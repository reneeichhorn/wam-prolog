use std::collections::VecDeque;

use crate::instructions::Instruction;

#[derive(Clone, Debug)]
pub struct Interpreter {
    pub global_stack: Vec<Cell>,
    pub registers: Vec<Cell>,
    instructions: Vec<Instruction>,
    pub instruction_index: usize,
    pub mode: Mode,
    pub next_sub_term_address: usize,
    pub execution_state: ExecutionState,
    functor_descriptions: Vec<FunctorDescription>,
}

#[derive(Clone, Debug)]
pub struct FunctorDescription {
    pub arity: usize,
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
    Structure(usize),
    Reference(usize),
    Undefined,
}

impl Interpreter {
    pub fn new(
        instructions: Vec<Instruction>,
        registers: usize,
        functor_descriptions: Vec<FunctorDescription>,
    ) -> Self {
        Self {
            global_stack: Vec::with_capacity(1024),
            registers: vec![Cell::Undefined; registers],
            instruction_index: 0,
            execution_state: ExecutionState::Normal,
            mode: Mode::Write,
            next_sub_term_address: 0,
            functor_descriptions,
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

    fn bind_address(&mut self, address: CellAddress, address_value: usize) {
        *self.lookup_address_mut(address) = Cell::Reference(address_value);
    }

    fn deref_cell(&self, address: CellAddress) -> CellAddress {
        let value = self.lookup_address(address);
        let index = address.index_num();
        match value {
            Cell::Reference(child_address) if *child_address != index => {
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
            let a = self.deref_cell(a);
            let b = self.deref_cell(b);
            if a == b {
                continue;
            }

            let a = self.lookup_address(a);
            let b = self.lookup_address(b);
            match (a, b) {
                (Cell::Reference(a_ref), Cell::Reference(b_ref)) => {
                    self.bind_address(CellAddress::GlobalStack { index: *a_ref }, *b_ref);
                }
                (Cell::StructureRef(a_ref), Cell::StructureRef(b_ref)) => {
                    let structure_a =
                        self.lookup_address(CellAddress::GlobalStack { index: *a_ref });
                    let structure_b =
                        self.lookup_address(CellAddress::GlobalStack { index: *b_ref });

                    match (structure_a, structure_b) {
                        (Cell::Structure(structure_a), Cell::Structure(structure_b)) => {
                            if *structure_a == *structure_b {
                                let functor_description = &self.functor_descriptions[*structure_a];
                                for i in 1..functor_description.arity {
                                    working_stack
                                        .push_back(CellAddress::GlobalStack { index: a_ref + i });
                                    working_stack
                                        .push_back(CellAddress::GlobalStack { index: b_ref + i });
                                }
                            } else {
                                self.execution_state = ExecutionState::Failure;
                                break;
                            }
                        }
                        _ => panic!(
                            "Unexpected execution assumption mismatch, STR referenced NOT a structure."
                        ),
                    }
                }
                _ => {}
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
                self.global_stack.push(Cell::Structure(structure.0));
                self.registers[*register] = Cell::StructureRef(self.global_stack.len() - 1);
            }
            Instruction::SetVariable { register } => {
                self.global_stack
                    .push(Cell::Reference(self.global_stack.len()));
                self.registers[*register] = Cell::Reference(self.global_stack.len() - 1);
            }
            Instruction::SetValue { register } => {
                let value = self.registers[*register].clone();
                self.global_stack.push(value);
            }
            // Debug instructions --------------------------------------------
            Instruction::DebugComment { .. } => {}
            // Program instructions --------------------------------------------
            Instruction::GetStructure {
                structure,
                register,
            } => {
                let address = self.deref_cell(CellAddress::Register { index: *register });
                let value = self.lookup_address(address);
                match value {
                    Cell::Reference(reference_index) => {
                        self.global_stack
                            .push(Cell::StructureRef(self.global_stack.len() + 1));
                        self.global_stack.push(Cell::Structure(structure.0));
                        self.bind_address(address, self.global_stack.len() - 2);
                        self.mode = Mode::Write;
                    }
                    Cell::StructureRef(structure_addr) => {
                        let target_structure = self.lookup_address(CellAddress::GlobalStack {
                            index: *structure_addr,
                        });
                        if target_structure == &Cell::Structure(structure.0) {
                            self.next_sub_term_address = structure_addr + 1;
                            self.mode = Mode::Read;
                        } else {
                            self.execution_state = ExecutionState::Failure;
                        }
                    }
                    _ => self.execution_state = ExecutionState::Failure,
                }
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
                            self.lookup_address_mut(CellAddress::Register { index: *register });
                        *register = next_sub_term_value;
                    }
                    Mode::Write => {
                        let new_value = Cell::Reference(self.global_stack.len());
                        self.global_stack.push(new_value.clone());
                        *self.lookup_address_mut(CellAddress::Register { index: *register }) =
                            new_value;
                    }
                }
                self.next_sub_term_address += 1;
            }
            Instruction::UnifyValue { register } => {
                match self.mode {
                    Mode::Read => {
                        self.unify(
                            CellAddress::Register { index: *register },
                            CellAddress::GlobalStack {
                                index: self.next_sub_term_address,
                            },
                        );
                    }
                    Mode::Write => {
                        let register =
                            self.lookup_address(CellAddress::Register { index: *register });
                        self.global_stack.push(register.clone());
                    }
                }
                self.next_sub_term_address += 1;
            }
        }

        true
    }
}
