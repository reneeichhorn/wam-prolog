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
    PutVariable {
        argument_register: RegisterId,
        variable_register: RegisterId,
    },
    PutValue {
        argument_register: RegisterId,
        value_register: RegisterId,
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
    GetVariable {
        argument_register: RegisterId,
        variable_register: RegisterId,
    },
    GetValue {
        argument_register: RegisterId,
        value_register: RegisterId,
    },
    UnifyVariable {
        register: RegisterId,
    },
    UnifyValue {
        register: RegisterId,
    },
    // Control Instructions ----------------------------
    Call {
        address: usize,
    },
    Proceed,
}
