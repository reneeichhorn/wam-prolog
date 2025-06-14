#[derive(Debug, Clone)]
pub struct StructureRef(pub usize);

#[derive(Debug, Clone)]
pub enum Instruction {
    // Query instructions ----------------------------
    PutStructure {
        structure: StructureRef,
        register: usize,
    },
    SetVariable {
        register: usize,
    },
    SetValue {
        register: usize,
    },
    DebugComment {
        message: Box<String>,
    },
    // Program instructions ----------------------------
    GetStructure {
        structure: StructureRef,
        register: usize,
    },
    UnifyVariable {
        register: usize,
    },
    UnifyValue {
        register: usize,
    },
}
