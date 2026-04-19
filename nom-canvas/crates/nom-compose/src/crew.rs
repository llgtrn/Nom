#![deny(unsafe_code)]

use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use crate::chain::ComposeRunnable;
use crate::context::ComposeContext;

// ---------------------------------------------------------------------------
// SharedMemory
// ---------------------------------------------------------------------------

/// Thread-safe shared memory for passing context between flow tasks.
#[derive(Clone)]
pub struct SharedMemory {
    data: Arc<Mutex<HashMap<String, String>>>,
}

impl SharedMemory {
    pub fn new() -> Self {
        Self {
            data: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn get(&self, key: &str) -> Option<String> {
        self.data.lock().unwrap().get(key).cloned()
    }

    pub fn set(&self, key: &str, value: String) {
        self.data.lock().unwrap().insert(key.to_string(), value);
    }

    pub fn snapshot(&self) -> HashMap<String, String> {
        self.data.lock().unwrap().clone()
    }
}

impl Default for SharedMemory {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Process
// ---------------------------------------------------------------------------

/// How a crew executes its tasks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Process {
    /// Execute tasks one at a time in order.
    Sequential,
    /// Execute tasks concurrently.
    Parallel,
}

// ---------------------------------------------------------------------------
// Agent
// ---------------------------------------------------------------------------

/// A single agent with a role, goal, and available tools.
pub struct Agent {
    pub role: String,
    pub goal: String,
    pub backstory: String,
    pub tools: Vec<Box<dyn ComposeRunnable>>,
    pub allow_delegation: bool,
}

impl Agent {
    pub fn new(role: impl Into<String>, goal: impl Into<String>, backstory: impl Into<String>) -> Self {
        Self {
            role: role.into(),
            goal: goal.into(),
            backstory: backstory.into(),
            tools: Vec::new(),
            allow_delegation: false,
        }
    }

    pub fn with_tool(mut self, tool: Box<dyn ComposeRunnable>) -> Self {
        self.tools.push(tool);
        self
    }

    pub fn allow_delegation(mut self, allow: bool) -> Self {
        self.allow_delegation = allow;
        self
    }
}

// ---------------------------------------------------------------------------
// Task
// ---------------------------------------------------------------------------

/// A task assigned to an agent in the crew.
pub struct Task {
    pub description: String,
    pub agent_index: usize,
    pub expected_output: String,
    /// IDs of upstream tasks whose outputs should be interpolated into the
    /// prompt before execution.
    pub context: Vec<String>,
}

impl Task {
    pub fn new(
        description: impl Into<String>,
        agent_index: usize,
        expected_output: impl Into<String>,
    ) -> Self {
        Self {
            description: description.into(),
            agent_index,
            expected_output: expected_output.into(),
            context: Vec::new(),
        }
    }

    pub fn with_context(mut self, ids: Vec<String>) -> Self {
        self.context = ids;
        self
    }
}

// ---------------------------------------------------------------------------
// Flow
// ---------------------------------------------------------------------------

/// A directed edge between two task nodes in a flow.
pub struct FlowEdge {
    pub from: String,
    pub to: String,
}

/// A node in the flow DAG wrapping a [`Task`] with a stable identifier.
pub struct FlowNode {
    pub id: String,
    pub task: Task,
}

/// A DAG of [`Task`] nodes with edges representing execution order.
pub struct Flow {
    pub nodes: HashMap<String, FlowNode>,
    pub edges: Vec<FlowEdge>,
}

impl Flow {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: Vec::new(),
        }
    }

    /// Add a task node keyed by `id`.
    pub fn add_node(&mut self, id: impl Into<String>, task: Task) {
        let id = id.into();
        self.nodes.insert(id.clone(), FlowNode { id, task });
    }

    /// Add a dependency edge (`from` must execute before `to`).
    pub fn add_edge(&mut self, from: impl Into<String>, to: impl Into<String>) -> Result<(), String> {
        let from = from.into();
        let to = to.into();
        if !self.nodes.contains_key(&from) {
            return Err(format!("flow add_edge: unknown from node '{}'", from));
        }
        if !self.nodes.contains_key(&to) {
            return Err(format!("flow add_edge: unknown to node '{}'", to));
        }
        self.edges.push(FlowEdge { from, to });
        Ok(())
    }

    /// Kahn's algorithm: returns node IDs in topological execution order.
    /// If the graph contains a cycle the returned vector will be shorter than
    /// `nodes.len()`.
    pub fn topological_order(&self) -> Vec<String> {
        let mut in_degree: HashMap<&str, usize> = HashMap::new();
        let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();

        for id in self.nodes.keys() {
            in_degree.entry(id.as_str()).or_insert(0);
            adj.entry(id.as_str()).or_default();
        }

        for edge in &self.edges {
            *in_degree.entry(edge.to.as_str()).or_insert(0) += 1;
            adj.entry(edge.from.as_str())
                .or_default()
                .push(edge.to.as_str());
        }

        let mut queue: VecDeque<&str> = {
            let mut starts: Vec<&str> = in_degree
                .iter()
                .filter(|(_, &deg)| deg == 0)
                .map(|(&id, _)| id)
                .collect();
            starts.sort();
            starts.into()
        };

        let mut order: Vec<String> = Vec::new();
        while let Some(node) = queue.pop_front() {
            order.push(node.to_string());
            if let Some(neighbors) = adj.get(node) {
                let mut next: Vec<&str> = neighbors.clone();
                next.sort();
                for neighbor in next {
                    let deg = in_degree.entry(neighbor).or_insert(0);
                    *deg = deg.saturating_sub(1);
                    if *deg == 0 {
                        queue.push_back(neighbor);
                    }
                }
            }
        }

        order
    }
}

impl Default for Flow {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// FlowExecutor
// ---------------------------------------------------------------------------

/// Executes a [`Flow`] DAG in topological order, parallelising tasks within
/// each independence wave.
pub struct FlowExecutor;

impl FlowExecutor {
    pub fn new() -> Self {
        Self
    }

    /// Run every node in `flow` through its assigned agent in `crew`.
    ///
    /// * Tasks are executed in topological waves.
    /// * Within a wave all tasks run concurrently via `std::thread::scope`.
    /// * Before a task runs, its [`Task::context`] IDs are resolved against
    ///   `memory` and appended to the prompt.
    /// * Each task output is written back to `memory` under its node ID.
    pub fn execute(
        &self,
        crew: &Crew,
        flow: &Flow,
        ctx: &ComposeContext,
        memory: &SharedMemory,
    ) -> Result<HashMap<String, String>, String> {
        let order = flow.topological_order();
        if order.len() != flow.nodes.len() {
            return Err("flow contains cycles".to_string());
        }

        // Pre-validate agent indices.
        for (id, node) in &flow.nodes {
            if node.task.agent_index >= crew.agents.len() {
                return Err(format!(
                    "flow node '{}' references invalid agent index {}",
                    id, node.task.agent_index
                ));
            }
        }

        // Build in-degree map for wave detection.
        let mut in_degree: HashMap<String, usize> = HashMap::new();
        let mut adj: HashMap<String, Vec<String>> = HashMap::new();
        for edge in &flow.edges {
            *in_degree.entry(edge.to.clone()).or_insert(0) += 1;
            adj.entry(edge.from.clone())
                .or_default()
                .push(edge.to.clone());
        }
        for id in flow.nodes.keys() {
            in_degree.entry(id.clone()).or_insert(0);
        }

        let mut completed: HashMap<String, String> = HashMap::new();
        let mut remaining: Vec<String> = order;

        while !remaining.is_empty() {
            // Every node whose in-degree is zero can run now.
            let wave: Vec<String> = remaining
                .iter()
                .filter(|id| *in_degree.get(*id).unwrap_or(&0) == 0)
                .cloned()
                .collect();

            if wave.is_empty() {
                return Err("flow cycle detected during execution".to_string());
            }

            remaining.retain(|id| !wave.contains(id));

            // Execute the wave in parallel.
            let wave_outputs: Vec<Result<(String, String), String>> = std::thread::scope(|s| {
                let mut handles = Vec::new();
                for node_id in &wave {
                    let node = &flow.nodes[node_id];
                    let agent = &crew.agents[node.task.agent_index];
                    let tools: &Vec<Box<dyn ComposeRunnable>> = &agent.tools;
                    let ctx = ctx.clone();
                    let memory = memory.clone();

                    // Interpolate upstream outputs from shared memory.
                    let mut desc = node.task.description.clone();
                    for ctx_id in &node.task.context {
                        if let Some(prev) = memory.get(ctx_id) {
                            desc.push_str(&format!(
                                "\n[Context from {}]: {}",
                                ctx_id, prev
                            ));
                        }
                    }

                    handles.push(s.spawn(move || {
                        let mut output = desc;
                        for tool in tools {
                            output = tool.run(&output, &ctx)?;
                        }
                        Ok((node_id.clone(), output))
                    }));
                }

                handles
                    .into_iter()
                    .map(|h| h.join().map_err(|_| "thread panicked".to_string())?)
                    .collect()
            });

            for res in wave_outputs {
                let (id, output) = res?;
                memory.set(&id, output.clone());
                completed.insert(id, output);
            }

            // Decrement in-degrees for the next wave.
            for node_id in &wave {
                if let Some(neighbors) = adj.get(node_id) {
                    for neighbor in neighbors {
                        if let Some(deg) = in_degree.get_mut(neighbor) {
                            *deg = deg.saturating_sub(1);
                        }
                    }
                }
            }
        }

        Ok(completed)
    }
}

// ---------------------------------------------------------------------------
// Crew
// ---------------------------------------------------------------------------

/// A team of agents with assigned tasks.
pub struct Crew {
    pub agents: Vec<Agent>,
    pub tasks: Vec<Task>,
    pub process: Process,
}

impl Crew {
    pub fn new(process: Process) -> Self {
        Self {
            agents: Vec::new(),
            tasks: Vec::new(),
            process,
        }
    }

    pub fn add_agent(mut self, agent: Agent) -> Self {
        self.agents.push(agent);
        self
    }

    pub fn add_task(mut self, task: Task) -> Self {
        self.tasks.push(task);
        self
    }

    /// Execute all tasks and return a map of task index to result.
    pub fn run(&self, ctx: &ComposeContext) -> Result<HashMap<usize, String>, String> {
        let mut results = HashMap::new();

        match self.process {
            Process::Sequential => {
                for (i, task) in self.tasks.iter().enumerate() {
                    let agent = self
                        .agents
                        .get(task.agent_index)
                        .ok_or_else(|| format!("no agent at index {}", task.agent_index))?;

                    let mut desc = task.description.clone();
                    for ctx_id in &task.context {
                        if let Some(prev) = ctx_id
                            .parse::<usize>()
                            .ok()
                            .and_then(|idx| results.get(&idx))
                        {
                            desc.push_str(&format!(
                                "\n[Context from task {}]: {}",
                                ctx_id, prev
                            ));
                        }
                    }

                    let mut output = desc;
                    for tool in &agent.tools {
                        output = tool.run(&output, ctx)?;
                    }
                    results.insert(i, output);
                }
                Ok(results)
            }
            Process::Parallel => {
                // Pre-validate agent indices to avoid borrow issues inside scope.
                for task in &self.tasks {
                    if task.agent_index >= self.agents.len() {
                        return Err(format!("no agent at index {}", task.agent_index));
                    }
                }

                let parallel_results: HashMap<usize, String> = std::thread::scope(|s| {
                    let mut handles = Vec::new();
                    for (i, task) in self.tasks.iter().enumerate() {
                        let tools: &Vec<Box<dyn ComposeRunnable>> = &self.agents[task.agent_index].tools;
                        let ctx = ctx.clone();
                        let desc = task.description.clone();

                        handles.push(s.spawn(move || {
                            let mut output = desc;
                            for tool in tools {
                                output = tool.run(&output, &ctx)?;
                            }
                            Ok::<_, String>((i, output))
                        }));
                    }

                    let mut local_results = HashMap::new();
                    for handle in handles {
                        let (i, output) =
                            handle.join().map_err(|_| "thread panicked".to_string())??;
                        local_results.insert(i, output);
                    }
                    Ok::<_, String>(local_results)
                })?;

                results = parallel_results;
                Ok(results)
            }
        }
    }

    /// Execute a [`Flow`] DAG using the agents in this crew.
    pub fn run_flow(
        &self,
        flow: &Flow,
        ctx: &ComposeContext,
        memory: &SharedMemory,
    ) -> Result<HashMap<String, String>, String> {
        FlowExecutor::new().execute(self, flow, ctx, memory)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chain::{LlmRunnable, ValidateRunnable};
    use crate::glue::StubLlmFn;
    use std::sync::Arc;

    // -----------------------------------------------------------------------
    // Existing behaviour
    // -----------------------------------------------------------------------

    #[test]
    fn test_crew_sequential_run() {
        let agent = Agent::new("composer", "compose media", "expert media composer")
            .with_tool(Box::new(ValidateRunnable::new()));
        let crew = Crew::new(Process::Sequential)
            .add_agent(agent)
            .add_task(Task::new("create video", 0, "mp4"));
        let results = crew.run(&ComposeContext::new("video", "scene")).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[&0], "create video");
    }

    #[test]
    fn test_crew_agent_has_role() {
        let agent = Agent::new("tester", "test code", "qa engineer");
        assert_eq!(agent.role, "tester");
        assert_eq!(agent.goal, "test code");
    }

    #[test]
    fn test_crew_task_assignment() {
        let crew = Crew::new(Process::Sequential)
            .add_agent(Agent::new("a1", "g1", "b1"))
            .add_agent(Agent::new("a2", "g2", "b2"))
            .add_task(Task::new("t1", 1, "o1"));
        assert_eq!(crew.tasks[0].agent_index, 1);
    }

    #[test]
    fn test_crew_parallel_process() {
        let crew = Crew::new(Process::Parallel)
            .add_agent(Agent::new("worker", "work", "hard worker"))
            .add_task(Task::new("job1", 0, "done"))
            .add_task(Task::new("job2", 0, "done"));
        let results = crew.run(&ComposeContext::new("test", "input")).unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_crew_empty_tasks() {
        let crew = Crew::new(Process::Sequential)
            .add_agent(Agent::new("idle", "wait", "patient"));
        let results = crew.run(&ComposeContext::new("test", "input")).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_crew_missing_agent_errors() {
        let crew = Crew::new(Process::Sequential).add_task(Task::new("orphan", 5, "fail"));
        let result = crew.run(&ComposeContext::new("test", "input"));
        assert!(result.is_err());
    }

    #[test]
    fn test_crew_tool_chain_execution() {
        let agent = Agent::new("writer", "write text", "copywriter").with_tool(Box::new(
            LlmRunnable::new(Arc::new(StubLlmFn {
                response: "draft".to_string(),
            })),
        ));
        let crew = Crew::new(Process::Sequential)
            .add_agent(agent)
            .add_task(Task::new("prompt", 0, "draft"));
        let results = crew.run(&ComposeContext::new("test", "input")).unwrap();
        assert_eq!(results[&0], "draft");
    }

    // -----------------------------------------------------------------------
    // New: Agent / Task / Memory primitives
    // -----------------------------------------------------------------------

    #[test]
    fn test_agent_allow_delegation() {
        let agent = Agent::new("manager", "manage team", "experienced manager")
            .allow_delegation(true);
        assert!(agent.allow_delegation);
    }

    #[test]
    fn test_task_with_context() {
        let task = Task::new("summarize", 0, "summary")
            .with_context(vec!["task_1".to_string(), "task_2".to_string()]);
        assert_eq!(task.context, vec!["task_1", "task_2"]);
    }

    #[test]
    fn test_shared_memory_get_set() {
        let mem = SharedMemory::new();
        mem.set("key1", "value1".to_string());
        assert_eq!(mem.get("key1"), Some("value1".to_string()));
        assert_eq!(mem.get("missing"), None);
    }

    #[test]
    fn test_shared_memory_snapshot() {
        let mem = SharedMemory::new();
        mem.set("a", "1".to_string());
        mem.set("b", "2".to_string());
        let snap = mem.snapshot();
        assert_eq!(snap.len(), 2);
        assert_eq!(snap["a"], "1");
        assert_eq!(snap["b"], "2");
    }

    // -----------------------------------------------------------------------
    // New: Flow graph
    // -----------------------------------------------------------------------

    #[test]
    fn test_flow_add_node_and_edge() {
        let mut flow = Flow::new();
        flow.add_node("n1", Task::new("task1", 0, "out1"));
        flow.add_node("n2", Task::new("task2", 0, "out2"));
        assert!(flow.add_edge("n1", "n2").is_ok());
        assert_eq!(flow.edges.len(), 1);
    }

    #[test]
    fn test_flow_add_edge_unknown_node_fails() {
        let mut flow = Flow::new();
        flow.add_node("n1", Task::new("task1", 0, "out1"));
        let err = flow.add_edge("n1", "ghost").unwrap_err();
        assert!(err.contains("ghost"));
    }

    #[test]
    fn test_flow_topological_order_linear() {
        let mut flow = Flow::new();
        flow.add_node("a", Task::new("A", 0, "out_a"));
        flow.add_node("b", Task::new("B", 0, "out_b"));
        flow.add_node("c", Task::new("C", 0, "out_c"));
        flow.add_edge("a", "b").unwrap();
        flow.add_edge("b", "c").unwrap();

        let order = flow.topological_order();
        assert_eq!(order, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_flow_topological_order_diamond() {
        let mut flow = Flow::new();
        flow.add_node("start", Task::new("Start", 0, "out_start"));
        flow.add_node("left", Task::new("Left", 0, "out_left"));
        flow.add_node("right", Task::new("Right", 0, "out_right"));
        flow.add_node("end", Task::new("End", 0, "out_end"));
        flow.add_edge("start", "left").unwrap();
        flow.add_edge("start", "right").unwrap();
        flow.add_edge("left", "end").unwrap();
        flow.add_edge("right", "end").unwrap();

        let order = flow.topological_order();
        assert_eq!(order[0], "start");
        assert_eq!(order[3], "end");
        let pos = |id: &str| order.iter().position(|x| x == id).unwrap();
        assert!(pos("start") < pos("left"));
        assert!(pos("start") < pos("right"));
        assert!(pos("left") < pos("end"));
        assert!(pos("right") < pos("end"));
    }

    #[test]
    fn test_flow_cycle_detected() {
        let mut flow = Flow::new();
        flow.add_node("a", Task::new("A", 0, "out_a"));
        flow.add_node("b", Task::new("B", 0, "out_b"));
        flow.add_edge("a", "b").unwrap();
        flow.add_edge("b", "a").unwrap();

        let order = flow.topological_order();
        assert_eq!(order.len(), 0);
    }

    // -----------------------------------------------------------------------
    // New: FlowExecutor
    // -----------------------------------------------------------------------

    #[test]
    fn test_flow_executor_linear() {
        let agent = Agent::new("worker", "work", "hard worker")
            .with_tool(Box::new(ValidateRunnable::new()));
        let crew = Crew::new(Process::Sequential).add_agent(agent);

        let mut flow = Flow::new();
        flow.add_node("step1", Task::new("hello", 0, "greeting"));
        flow.add_node(
            "step2",
            Task::new("world", 0, "noun").with_context(vec!["step1".to_string()]),
        );
        flow.add_edge("step1", "step2").unwrap();

        let memory = SharedMemory::new();
        let results = crew
            .run_flow(&flow, &ComposeContext::new("test", "input"), &memory)
            .unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results["step1"], "hello");
        assert!(results["step2"].contains("hello"));
        assert!(results["step2"].contains("world"));
    }

    #[test]
    fn test_flow_executor_diamond_with_tools() {
        let researcher = Agent::new("researcher", "research", "expert researcher").with_tool(
            Box::new(LlmRunnable::new(Arc::new(StubLlmFn {
                response: "research_data".to_string(),
            }))),
        );
        let writer = Agent::new("writer", "write", "expert writer").with_tool(Box::new(
            LlmRunnable::new(Arc::new(StubLlmFn {
                response: "draft".to_string(),
            })),
        ));
        let editor = Agent::new("editor", "edit", "expert editor").with_tool(Box::new(
            LlmRunnable::new(Arc::new(StubLlmFn {
                response: "final".to_string(),
            })),
        ));

        let crew = Crew::new(Process::Sequential)
            .add_agent(researcher)
            .add_agent(writer)
            .add_agent(editor);

        let mut flow = Flow::new();
        flow.add_node("research", Task::new("do research", 0, "data"));
        flow.add_node(
            "outline",
            Task::new("write outline", 1, "outline").with_context(vec!["research".to_string()]),
        );
        flow.add_node(
            "draft",
            Task::new("write draft", 1, "draft").with_context(vec!["research".to_string()]),
        );
        flow.add_node(
            "edit",
            Task::new("edit document", 2, "final")
                .with_context(vec!["outline".to_string(), "draft".to_string()]),
        );

        flow.add_edge("research", "outline").unwrap();
        flow.add_edge("research", "draft").unwrap();
        flow.add_edge("outline", "edit").unwrap();
        flow.add_edge("draft", "edit").unwrap();

        let memory = SharedMemory::new();
        let results = crew
            .run_flow(&flow, &ComposeContext::new("test", "input"), &memory)
            .unwrap();
        assert_eq!(results.len(), 4);
        assert_eq!(results["research"], "research_data");
        assert_eq!(results["outline"], "draft");
        assert_eq!(results["draft"], "draft");
        assert_eq!(results["edit"], "final");
    }

    #[test]
    fn test_flow_executor_missing_agent_errors() {
        let crew = Crew::new(Process::Sequential);
        let mut flow = Flow::new();
        flow.add_node("bad", Task::new("bad task", 0, "fail"));

        let memory = SharedMemory::new();
        let result = crew.run_flow(&flow, &ComposeContext::new("test", "input"), &memory);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("invalid agent index"));
    }

    #[test]
    fn test_flow_executor_cycle_errors() {
        let crew = Crew::new(Process::Sequential).add_agent(Agent::new("a", "g", "b"));
        let mut flow = Flow::new();
        flow.add_node("x", Task::new("X", 0, "out_x"));
        flow.add_node("y", Task::new("Y", 0, "out_y"));
        flow.add_edge("x", "y").unwrap();
        flow.add_edge("y", "x").unwrap();

        let memory = SharedMemory::new();
        let result = crew.run_flow(&flow, &ComposeContext::new("test", "input"), &memory);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("cycles"));
    }

    #[test]
    fn test_crew_task_context_chaining() {
        let agent = Agent::new("writer", "write", "copywriter")
            .with_tool(Box::new(ValidateRunnable::new()));
        let crew = Crew::new(Process::Sequential)
            .add_agent(agent)
            .add_task(Task::new("first draft", 0, "draft"))
            .add_task(
                Task::new("final version", 0, "final").with_context(vec!["0".to_string()]),
            );

        let results = crew.run(&ComposeContext::new("test", "input")).unwrap();
        assert_eq!(results.len(), 2);
        assert!(results[&1].contains("first draft"));
    }
}
