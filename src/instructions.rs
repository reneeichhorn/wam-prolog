#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
pub struct DescriptorId(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RegisterId(pub usize);

#[derive(Debug, Clone)]
pub enum Instruction {
    // Query instructions ----------------------------
    PutStructure {
        structure: DescriptorId,
        register: RegisterId,
    },
    SetVariable {
        register: RegisterId,
    },
    SetValue {
        register: RegisterId,
    },
    DebugComment {
        message: Box<String>,
    },
    // Program instructions ----------------------------
    GetStructure {
        structure: DescriptorId,
        register: RegisterId,
    },
    UnifyVariable {
        register: RegisterId,
    },
    UnifyValue {
        register: RegisterId,
    },
}
