use std::collections::VecDeque;

use crate::{
    descriptor::TermDescriptor,
    instructions::{DescriptorId, Instruction},
};

#[derive(Clone, Debug)]
pub struct Interpreter {
    pub global_stack: Vec<Cell>,
    pub registers: Vec<Cell>,
    instructions: Vec<Instruction>,
    pub instruction_index: usize,
    pub mode: Mode,
    pub next_sub_term_address: usize,
    pub execution_state: ExecutionState,
    descriptors: Vec<TermDescriptor>,
}

#[derive(Clone, Debug)]
pub enum Mode {
    Read,
    Write,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExecutionState {
    Normal,
    Failure,
}

#[derive(Copy, Debug, Clone, PartialEq, Eq)]
pub enum CellAddress {
    Register { index: usize },
    GlobalStack { index: usize },
}

impl CellAddress {
    fn is_global_stack(&self) -> bool {
        match self {
            CellAddress::Register { .. } => false,
            CellAddress::GlobalStack { .. } => true,
        }
    }

    fn is_register(&self) -> bool {
        match self {
            CellAddress::Register { .. } => true,
            CellAddress::GlobalStack { .. } => false,
        }
    }

    fn index_num(&self) -> usize {
        match self {
            CellAddress::Register { index } => *index,
            CellAddress::GlobalStack { index } => *index,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Cell {
    StructureRef(usize),
    Structure(DescriptorId),
    Reference(usize),
    Undefined,
}

impl Interpreter {
    pub fn new(
        instructions: Vec<Instruction>,
        registers: usize,
        descriptors: Vec<TermDescriptor>,
    ) -> Self {
        Self {
            global_stack: Vec::with_capacity(1024),
            registers: vec![Cell::Undefined; registers],
            instruction_index: 0,
            execution_state: ExecutionState::Normal,
            mode: Mode::Write,
            next_sub_term_address: 0,
            descriptors,
            instructions,
        }
    }

    fn lookup_address(&self, address: CellAddress) -> &Cell {
        match address {
            CellAddress::Register { index } => &self.registers[index],
            CellAddress::GlobalStack { index } => &self.global_stack[index],
        }
    }

    fn lookup_address_mut(&mut self, address: CellAddress) -> &mut Cell {
        match address {
            CellAddress::Register { index } => &mut self.registers[index],
            CellAddress::GlobalStack { index } => &mut self.global_stack[index],
        }
    }

    fn bind_address(&mut self, a: CellAddress, b: CellAddress) {
        let b_address = b.index_num();

        let a = self.deref_cell(a);
        let a_cell = self.lookup_address_mut(a);
        *a_cell = Cell::Reference(b_address);

        /*
        let a = self.deref_cell(a);
        let b = self.deref_cell(b);

        let a_value = self.lookup_address(a);
        let b_value = self.lookup_address(b);

        match a_value {
            Cell::Reference(a_reference) => {
                let new_value = b.index_num();
                let a_value_mut = self.lookup_address_mut(a);
                *a_value_mut = Cell::Reference(new_value);
            }
            _ => {
                let new_value = a.index_num();
                let b_value_mut = self.lookup_address_mut(b);
                *b_value_mut = Cell::Reference(new_value);
            }
        }
        match (a_value, b_value) {
            (Cell::Reference(a_ref), Cell::Reference(b_ref)) if a_ref < b_ref => {
                let new_value = b_value.clone();
                let a_value_mut = self.lookup_address_mut(a);
                *a_value_mut = new_value;
            }
            (Cell::Reference(_), _) => {
                let new_value = b_value.clone();
                let a_value_mut = self.lookup_address_mut(a);
                *a_value_mut = new_value;
            }
            _ => {
                let new_value = a_value.clone();
                let b_value_mut = self.lookup_address_mut(b);
                *b_value_mut = new_value;
            }
        }
        */
    }

    fn deref_cell(&self, address: CellAddress) -> CellAddress {
        let value = self.lookup_address(address);
        let index = address.index_num();
        match value {
            Cell::Reference(child_address) if *child_address != index || address.is_register() => {
                self.deref_cell(CellAddress::GlobalStack {
                    index: *child_address,
                })
            }
            _ => address,
        }
    }

    fn unify(&mut self, a: CellAddress, b: CellAddress) {
        let mut working_stack = VecDeque::new();
        working_stack.push_back(a);
        working_stack.push_back(b);

        while let (Some(a), Some(b)) = (working_stack.pop_back(), working_stack.pop_back()) {
            let a_address = self.deref_cell(a);
            let b_address = self.deref_cell(b);
            if a == b {
                continue;
            }

            let a = self.lookup_address(a_address);
            let b = self.lookup_address(b_address);

            match (a, b) {
                (Cell::Reference(_), _) | (_, Cell::Reference(_)) => {
                    self.bind_address(a_address, b_address);
                }
                (Cell::StructureRef(a_ref), Cell::StructureRef(b_ref)) => {
                    let structure_a =
                        self.lookup_address(CellAddress::GlobalStack { index: *a_ref });
                    let structure_b =
                        self.lookup_address(CellAddress::GlobalStack { index: *b_ref });

                    match (structure_a, structure_b) {
                        (Cell::Structure(structure_a), Cell::Structure(structure_b)) => {
                            if *structure_a == *structure_b {
                                let functor_description = &self.descriptors[structure_a.0];
                                for i in 1..functor_description.arity() {
                                    working_stack
                                        .push_back(CellAddress::GlobalStack { index: a_ref + i });
                                    working_stack
                                        .push_back(CellAddress::GlobalStack { index: b_ref + i });
                                }
                                continue;
                            }
                        }
                        _ => {}
                    }

                    self.execution_state = ExecutionState::Failure;
                    break;
                }
                _ => {
                    self.execution_state = ExecutionState::Failure;
                    break;
                }
            }
        }
    }

    pub fn step(&mut self) -> bool {
        if self.execution_state == ExecutionState::Failure {
            return false;
        }
        if self.instruction_index == self.instructions.len() {
            return false;
        }
        let instruction = &self.instructions[self.instruction_index];
        self.instruction_index += 1;

        match instruction {
            // Query instructions --------------------------------------------
            Instruction::PutStructure {
                structure,
                register,
            } => {
                self.global_stack
                    .push(Cell::StructureRef(self.global_stack.len() + 1));
                self.global_stack.push(Cell::Structure(*structure));
                self.registers[register.0] = Cell::StructureRef(self.global_stack.len() - 1);
            }
            Instruction::SetVariable { register } => {
                self.global_stack
                    .push(Cell::Reference(self.global_stack.len()));
                self.registers[register.0] = Cell::Reference(self.global_stack.len() - 1);
            }
            Instruction::SetValue { register } => {
                let value = self.registers[register.0].clone();
                self.global_stack.push(value);
            }
            Instruction::PutValue {
                value_register,
                argument_register,
            } => {
                let value = self.registers[value_register.0].clone();
                self.registers[argument_register.0] = value;
            }
            Instruction::PutVariable {
                argument_register,
                variable_register,
            } => {
                let new_unbound = Cell::Reference(self.global_stack.len());
                self.global_stack.push(new_unbound.clone());
                self.registers[argument_register.0] = new_unbound.clone();
                self.registers[variable_register.0] = new_unbound;
            }

            // Debug instructions --------------------------------------------
            Instruction::DebugComment { .. } => {}
            // Program instructions --------------------------------------------
            Instruction::GetStructure {
                structure,
                register,
            } => {
                let address = self.deref_cell(CellAddress::Register { index: register.0 });
                let value = self.lookup_address(address);
                match value {
                    Cell::Reference(reference_index) => {
                        self.global_stack
                            .push(Cell::StructureRef(self.global_stack.len() + 1));
                        self.global_stack.push(Cell::Structure(*structure));
                        self.bind_address(
                            address,
                            CellAddress::GlobalStack {
                                index: self.global_stack.len() - 2,
                            },
                        );
                        self.mode = Mode::Write;
                    }
                    Cell::StructureRef(structure_addr) => {
                        let target_structure = self.lookup_address(CellAddress::GlobalStack {
                            index: *structure_addr,
                        });
                        if target_structure == &Cell::Structure(*structure) {
                            self.next_sub_term_address = structure_addr + 1;
                            self.mode = Mode::Read;
                        } else {
                            self.execution_state = ExecutionState::Failure;
                        }
                    }
                    _ => self.execution_state = ExecutionState::Failure,
                }
            }
            Instruction::GetVariable {
                argument_register,
                variable_register,
            } => {
                self.registers[variable_register.0] = self.registers[argument_register.0].clone();
            }
            Instruction::GetValue {
                argument_register,
                value_register,
            } => {
                self.unify(
                    CellAddress::Register {
                        index: value_register.0,
                    },
                    CellAddress::Register {
                        index: argument_register.0,
                    },
                );
            }
            Instruction::UnifyVariable { register } => {
                match self.mode {
                    Mode::Read => {
                        let next_sub_term_value = self
                            .lookup_address(CellAddress::GlobalStack {
                                index: self.next_sub_term_address,
                            })
                            .clone();

                        let register =
                            self.lookup_address_mut(CellAddress::Register { index: register.0 });
                        *register = next_sub_term_value;
                    }
                    Mode::Write => {
                        let new_value = Cell::Reference(self.global_stack.len());
                        self.global_stack.push(new_value.clone());
                        *self.lookup_address_mut(CellAddress::Register { index: register.0 }) =
                            new_value;
                    }
                }
                self.next_sub_term_address += 1;
            }
            Instruction::UnifyValue { register } => {
                match self.mode {
                    Mode::Read => {
                        self.unify(
                            CellAddress::Register { index: register.0 },
                            CellAddress::GlobalStack {
                                index: self.next_sub_term_address,
                            },
                        );
                    }
                    Mode::Write => {
                        let register =
                            self.lookup_address(CellAddress::Register { index: register.0 });
                        self.global_stack.push(register.clone());
                    }
                }
                self.next_sub_term_address += 1;
            }
            // Control flow
            Instruction::Proceed => {
                // TODO: proper proceed
                return false;
            }
            Instruction::Call { address } => {
                self.instruction_index = *address;
            }
        }

        true
    }
}
