//! SSA (Static Single Assignment) form stubs for IR basic block conversion.
//!
//! Provides `SsaVar`, `PhiNode`, `SsaBlock`, and `SsaForm` — the structural
//! primitives needed to convert a CFG of basic blocks into SSA form.

/// A variable in SSA form: a base name plus a version number.
#[derive(Debug, Clone, PartialEq)]
pub struct SsaVar {
    pub base_name: String,
    pub version: u32,
}

impl SsaVar {
    pub fn new(base_name: impl Into<String>, version: u32) -> Self {
        Self { base_name: base_name.into(), version }
    }

    /// Returns the canonical SSA name, e.g. `"x_v0"`.
    pub fn ssa_name(&self) -> String {
        format!("{}_v{}", self.base_name, self.version)
    }

    /// Returns a new `SsaVar` with `version + 1`.
    pub fn next_version(&self) -> SsaVar {
        SsaVar::new(self.base_name.clone(), self.version + 1)
    }
}

/// A φ-node placed at a basic block entry: selects among values from
/// predecessor blocks.
#[derive(Debug, Clone, PartialEq)]
pub struct PhiNode {
    pub result: SsaVar,
    /// `(predecessor_block_id, variable)` pairs.
    pub operands: Vec<(u64, SsaVar)>,
}

impl PhiNode {
    pub fn new(result: SsaVar) -> Self {
        Self { result, operands: Vec::new() }
    }

    pub fn add_operand(&mut self, block_id: u64, var: SsaVar) {
        self.operands.push((block_id, var));
    }

    pub fn operand_count(&self) -> usize {
        self.operands.len()
    }
}

/// One basic block in SSA form: holds its φ-nodes and the variables it defines.
#[derive(Debug, Clone, PartialEq)]
pub struct SsaBlock {
    pub id: u64,
    pub phi_nodes: Vec<PhiNode>,
    /// Variables defined (assigned) in this block.
    pub definitions: Vec<SsaVar>,
}

impl SsaBlock {
    pub fn new(id: u64) -> Self {
        Self { id, phi_nodes: Vec::new(), definitions: Vec::new() }
    }

    pub fn add_phi(&mut self, phi: PhiNode) {
        self.phi_nodes.push(phi);
    }

    pub fn define(&mut self, var: SsaVar) {
        self.definitions.push(var);
    }

    pub fn phi_count(&self) -> usize {
        self.phi_nodes.len()
    }
}

/// The complete SSA form of a function: an ordered collection of `SsaBlock`s.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SsaForm {
    pub blocks: Vec<SsaBlock>,
}

impl SsaForm {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_block(&mut self, block: SsaBlock) {
        self.blocks.push(block);
    }

    pub fn block_count(&self) -> usize {
        self.blocks.len()
    }

    /// Sum of φ-nodes across all blocks.
    pub fn total_phi_nodes(&self) -> usize {
        self.blocks.iter().map(|b| b.phi_nodes.len()).sum()
    }

    /// Search all blocks' `definitions` for a variable whose `ssa_name()` equals
    /// `ssa_name`. Returns the first match or `None`.
    pub fn find_definition(&self, ssa_name: &str) -> Option<&SsaVar> {
        self.blocks
            .iter()
            .flat_map(|b| b.definitions.iter())
            .find(|v| v.ssa_name() == ssa_name)
    }
}

#[cfg(test)]
mod ssa_tests {
    use super::*;

    #[test]
    fn ssa_var_ssa_name_format() {
        let v = SsaVar::new("x", 0);
        assert_eq!(v.ssa_name(), "x_v0");

        let v2 = SsaVar::new("counter", 3);
        assert_eq!(v2.ssa_name(), "counter_v3");
    }

    #[test]
    fn ssa_var_next_version_increments() {
        let v = SsaVar::new("y", 2);
        let next = v.next_version();
        assert_eq!(next.base_name, "y");
        assert_eq!(next.version, 3);
    }

    #[test]
    fn phi_node_add_operand_count() {
        let result = SsaVar::new("x", 1);
        let mut phi = PhiNode::new(result);
        assert_eq!(phi.operand_count(), 0);

        phi.add_operand(0, SsaVar::new("x", 0));
        phi.add_operand(1, SsaVar::new("x", 0));
        assert_eq!(phi.operand_count(), 2);
    }

    #[test]
    fn ssa_block_define_and_find_definition() {
        let mut block = SsaBlock::new(0);
        block.define(SsaVar::new("a", 0));
        block.define(SsaVar::new("b", 1));

        let mut form = SsaForm::new();
        form.add_block(block);

        let found = form.find_definition("a_v0");
        assert!(found.is_some());
        assert_eq!(found.unwrap().ssa_name(), "a_v0");
    }

    #[test]
    fn ssa_block_phi_count() {
        let mut block = SsaBlock::new(5);
        assert_eq!(block.phi_count(), 0);

        block.add_phi(PhiNode::new(SsaVar::new("z", 1)));
        block.add_phi(PhiNode::new(SsaVar::new("w", 0)));
        assert_eq!(block.phi_count(), 2);
    }

    #[test]
    fn ssa_form_block_count() {
        let mut form = SsaForm::new();
        assert_eq!(form.block_count(), 0);

        form.add_block(SsaBlock::new(0));
        form.add_block(SsaBlock::new(1));
        assert_eq!(form.block_count(), 2);
    }

    #[test]
    fn ssa_form_total_phi_nodes_sums() {
        let mut b0 = SsaBlock::new(0);
        b0.add_phi(PhiNode::new(SsaVar::new("x", 1)));

        let mut b1 = SsaBlock::new(1);
        b1.add_phi(PhiNode::new(SsaVar::new("y", 1)));
        b1.add_phi(PhiNode::new(SsaVar::new("z", 1)));

        let mut form = SsaForm::new();
        form.add_block(b0);
        form.add_block(b1);

        assert_eq!(form.total_phi_nodes(), 3);
    }

    #[test]
    fn find_definition_found_across_blocks() {
        let mut b0 = SsaBlock::new(0);
        b0.define(SsaVar::new("p", 0));

        let mut b1 = SsaBlock::new(1);
        b1.define(SsaVar::new("q", 2));

        let mut form = SsaForm::new();
        form.add_block(b0);
        form.add_block(b1);

        let found = form.find_definition("q_v2");
        assert!(found.is_some());
        assert_eq!(found.unwrap().base_name, "q");
        assert_eq!(found.unwrap().version, 2);
    }

    #[test]
    fn find_definition_none_for_missing() {
        let mut b0 = SsaBlock::new(0);
        b0.define(SsaVar::new("a", 0));

        let mut form = SsaForm::new();
        form.add_block(b0);

        assert!(form.find_definition("missing_v99").is_none());
    }
}
