//! LLVM IR emission types with op dispatch table (A11 coverage).

/// LLVM instruction opcodes with dispatch metadata.
#[derive(Debug, Clone, PartialEq)]
pub enum LlvmOp {
    Add,
    Sub,
    Mul,
    Div,
    Load,
    Store,
    Call,
    Return,
    Branch,
    Compare,
}

impl LlvmOp {
    /// LLVM IR mnemonic for this op.
    pub fn op_name(&self) -> &str {
        match self {
            LlvmOp::Add => "add",
            LlvmOp::Sub => "sub",
            LlvmOp::Mul => "mul",
            LlvmOp::Div => "div",
            LlvmOp::Load => "load",
            LlvmOp::Store => "store",
            LlvmOp::Call => "call",
            LlvmOp::Return => "ret",
            LlvmOp::Branch => "br",
            LlvmOp::Compare => "icmp",
        }
    }

    /// Number of operands this op expects.
    pub fn operand_count(&self) -> u8 {
        match self {
            LlvmOp::Add | LlvmOp::Sub | LlvmOp::Mul | LlvmOp::Div | LlvmOp::Compare => 2,
            LlvmOp::Load => 1,
            LlvmOp::Store => 2,
            LlvmOp::Call => 1,
            LlvmOp::Return => 0,
            LlvmOp::Branch => 1,
        }
    }

    /// True if this op terminates a basic block.
    pub fn is_terminator(&self) -> bool {
        matches!(self, LlvmOp::Return | LlvmOp::Branch)
    }
}

/// A single LLVM IR instruction.
#[derive(Debug, Clone, PartialEq)]
pub struct LlvmInstr {
    pub op: LlvmOp,
    pub result_reg: Option<String>,
    pub operands: Vec<String>,
}

impl LlvmInstr {
    pub fn new(
        op: LlvmOp,
        result_reg: Option<impl Into<String>>,
        operands: Vec<String>,
    ) -> Self {
        LlvmInstr {
            op,
            result_reg: result_reg.map(|r| r.into()),
            operands,
        }
    }

    /// Format as LLVM IR text.
    ///
    /// With result reg: `%reg = add i32 %a, %b`
    /// Without:         `ret void`
    pub fn emit(&self) -> String {
        let mnemonic = self.op.op_name();
        let operand_str = self.operands.join(", ");
        match &self.result_reg {
            Some(reg) => {
                if operand_str.is_empty() {
                    format!("{} = {}", reg, mnemonic)
                } else {
                    format!("{} = {} {}", reg, mnemonic, operand_str)
                }
            }
            None => {
                if operand_str.is_empty() {
                    format!("{} void", mnemonic)
                } else {
                    format!("{} {}", mnemonic, operand_str)
                }
            }
        }
    }
}

/// A basic block in an LLVM function.
#[derive(Debug, Clone)]
pub struct LlvmBlock {
    pub label: String,
    pub instrs: Vec<LlvmInstr>,
}

impl LlvmBlock {
    pub fn new(label: impl Into<String>) -> Self {
        LlvmBlock {
            label: label.into(),
            instrs: Vec::new(),
        }
    }

    pub fn add_instr(&mut self, instr: LlvmInstr) {
        self.instrs.push(instr);
    }

    pub fn instr_count(&self) -> usize {
        self.instrs.len()
    }

    /// True if the last instruction is a terminator.
    pub fn has_terminator(&self) -> bool {
        self.instrs.last().map(|i| i.op.is_terminator()).unwrap_or(false)
    }
}

/// An LLVM function composed of basic blocks.
#[derive(Debug, Clone)]
pub struct LlvmFunction {
    pub name: String,
    pub blocks: Vec<LlvmBlock>,
}

impl LlvmFunction {
    pub fn new(name: impl Into<String>) -> Self {
        LlvmFunction {
            name: name.into(),
            blocks: Vec::new(),
        }
    }

    pub fn add_block(&mut self, block: LlvmBlock) {
        self.blocks.push(block);
    }

    pub fn block_count(&self) -> usize {
        self.blocks.len()
    }

    /// Emit the full function as LLVM IR text.
    pub fn emit_ir(&self) -> String {
        let mut lines = Vec::new();
        lines.push(format!("define void @{}() {{", self.name));
        for block in &self.blocks {
            lines.push(format!("{}:", block.label));
            for instr in &block.instrs {
                lines.push(format!("  {}", instr.emit()));
            }
        }
        lines.push("}".to_string());
        lines.join("\n")
    }
}

#[cfg(test)]
mod llvm_emit_tests {
    use super::*;

    #[test]
    fn test_op_name() {
        assert_eq!(LlvmOp::Add.op_name(), "add");
        assert_eq!(LlvmOp::Sub.op_name(), "sub");
        assert_eq!(LlvmOp::Return.op_name(), "ret");
        assert_eq!(LlvmOp::Branch.op_name(), "br");
        assert_eq!(LlvmOp::Compare.op_name(), "icmp");
    }

    #[test]
    fn test_is_terminator() {
        assert!(LlvmOp::Return.is_terminator());
        assert!(LlvmOp::Branch.is_terminator());
        assert!(!LlvmOp::Add.is_terminator());
        assert!(!LlvmOp::Call.is_terminator());
        assert!(!LlvmOp::Load.is_terminator());
    }

    #[test]
    fn test_instr_emit_add() {
        let instr = LlvmInstr::new(
            LlvmOp::Add,
            Some("%0"),
            vec!["i32 %a".to_string(), "%b".to_string()],
        );
        assert_eq!(instr.emit(), "%0 = add i32 %a, %b");
    }

    #[test]
    fn test_instr_emit_ret_no_result_reg() {
        let instr = LlvmInstr::new(LlvmOp::Return, None::<String>, vec![]);
        assert_eq!(instr.emit(), "ret void");
    }

    #[test]
    fn test_block_add_instr_increments_count() {
        let mut block = LlvmBlock::new("entry");
        assert_eq!(block.instr_count(), 0);
        block.add_instr(LlvmInstr::new(
            LlvmOp::Add,
            Some("%0"),
            vec!["i32 1".to_string(), "i32 2".to_string()],
        ));
        assert_eq!(block.instr_count(), 1);
        block.add_instr(LlvmInstr::new(LlvmOp::Return, None::<String>, vec![]));
        assert_eq!(block.instr_count(), 2);
    }

    #[test]
    fn test_block_has_terminator_true_when_last_is_return() {
        let mut block = LlvmBlock::new("entry");
        block.add_instr(LlvmInstr::new(
            LlvmOp::Add,
            Some("%0"),
            vec!["i32 1".to_string(), "i32 2".to_string()],
        ));
        block.add_instr(LlvmInstr::new(LlvmOp::Return, None::<String>, vec![]));
        assert!(block.has_terminator());
    }

    #[test]
    fn test_block_has_terminator_false_for_empty_block() {
        let block = LlvmBlock::new("entry");
        assert!(!block.has_terminator());
    }

    #[test]
    fn test_function_block_count_after_add_block() {
        let mut func = LlvmFunction::new("main");
        assert_eq!(func.block_count(), 0);
        func.add_block(LlvmBlock::new("entry"));
        assert_eq!(func.block_count(), 1);
        func.add_block(LlvmBlock::new("exit"));
        assert_eq!(func.block_count(), 2);
    }

    #[test]
    fn test_function_emit_ir_contains_function_name() {
        let mut func = LlvmFunction::new("compute");
        let mut block = LlvmBlock::new("entry");
        block.add_instr(LlvmInstr::new(LlvmOp::Return, None::<String>, vec![]));
        func.add_block(block);
        let ir = func.emit_ir();
        assert!(ir.contains("@compute"));
        assert!(ir.contains("define void @compute()"));
    }
}
