#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
pub struct DescriptorId(pub usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RegisterId {
    Argument(usize),
    Temporary(usize),
    Permanent(usize),
}

impl RegisterId {
    pub fn index_num(&self) -> usize {
        match self {
            RegisterId::Argument(index) => *index,
            RegisterId::Temporary(index) => *index,
            RegisterId::Permanent(index) => *index,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Instruction {
    // Query instructions ----------------------------
    PutStructure {
        structure: DescriptorId,
        register: RegisterId,
    },
    PutVariable {
        argument_register: RegisterId,
        variable_register: RegisterId,
    },
    PutValue {
        argument_register: RegisterId,
        value_register: RegisterId,
    },
    PutConstant {
        constant: DescriptorId,
        register: RegisterId,
    },
    SetVariable {
        register: RegisterId,
    },
    SetValue {
        register: RegisterId,
    },
    SetConstant {
        constant: DescriptorId,
    },
    DebugComment {
        message: Box<String>,
    },
    // Program instructions ----------------------------
    GetStructure {
        structure: DescriptorId,
        register: RegisterId,
    },
    GetVariable {
        argument_register: RegisterId,
        variable_register: RegisterId,
    },
    GetValue {
        argument_register: RegisterId,
        value_register: RegisterId,
    },
    GetConstant {
        constant: DescriptorId,
        register: RegisterId,
    },
    UnifyVariable {
        register: RegisterId,
    },
    UnifyValue {
        register: RegisterId,
    },
    UnifyConstant {
        constant: DescriptorId,
    },
    // Control Instructions ----------------------------
    Call {
        address: usize,
        functor: DescriptorId,
    },
    Allocate {
        variables: usize,
    },
    Deallocate,
    Proceed,
    TryMeElse {
        else_address: usize,
    },
    RetryMeElse {
        else_address: usize,
    },
    TrustMe,
    NoOp,
}
