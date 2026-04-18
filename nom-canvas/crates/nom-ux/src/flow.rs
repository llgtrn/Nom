#[derive(Debug, Clone)]
pub struct FlowStep {
    pub index: u32,
    pub action: String,
    pub screen_id: String,
}

impl FlowStep {
    pub fn new(index: u32, action: &str, screen_id: &str) -> Self {
        Self {
            index,
            action: action.to_owned(),
            screen_id: screen_id.to_owned(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct UserFlow {
    pub id: String,
    pub name: String,
    pub steps: Vec<FlowStep>,
}

impl UserFlow {
    pub fn new(id: &str, name: &str) -> Self {
        Self {
            id: id.to_owned(),
            name: name.to_owned(),
            steps: Vec::new(),
        }
    }

    pub fn push_step(mut self, step: FlowStep) -> Self {
        self.steps.push(step);
        self
    }

    pub fn step_count(&self) -> usize {
        self.steps.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_flow_is_empty() {
        let f = UserFlow::new("f1", "Onboarding");
        assert_eq!(f.id, "f1");
        assert_eq!(f.name, "Onboarding");
        assert_eq!(f.step_count(), 0);
    }

    #[test]
    fn push_step_increments_count() {
        let f = UserFlow::new("f2", "Edit")
            .push_step(FlowStep::new(0, "open", "s1"))
            .push_step(FlowStep::new(1, "edit", "s2"))
            .push_step(FlowStep::new(2, "save", "s2"));
        assert_eq!(f.step_count(), 3);
        assert_eq!(f.steps[1].action, "edit");
    }
}
