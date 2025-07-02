use std::{
    collections::{HashMap, VecDeque},
    ops::Range,
};

use pest::Stack;

use crate::{
    descriptor::TermDescriptor,
    instructions::{DescriptorId, Instruction, RegisterId},
    interpreter::{choicepoint::ChoicePointStack, environment::EnvironmentStack},
};

mod choicepoint;
mod environment;

#[derive(Clone, Debug)]
pub struct Interpreter {
    pub global_stack: Vec<Cell>,
    pub registers: Vec<Cell>,
    pub trail: Vec<CellAddress>,
    instructions: Vec<Instruction>,
    pub instruction_index: usize,
    pub mode: Mode,
    pub next_sub_term_address: usize,
    pub execution_state: ExecutionState,
    pub environment_stack: EnvironmentStack,
    pub choice_point_stack: ChoicePointStack,
    pub proceed_return_address: usize,
    pub current_functor: DescriptorId,
    inspection_watch: Vec<WatchCell>,
    inspection_set: bool,
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
    Register { index: RegisterId },
    GlobalStack { index: usize },
}

impl CellAddress {
    fn is_register(&self) -> bool {
        match self {
            CellAddress::Register { .. } => true,
            CellAddress::GlobalStack { .. } => false,
        }
    }

    fn index_num(&self) -> usize {
        match self {
            CellAddress::Register { index } => match index {
                RegisterId::Argument(i) => *i,
                RegisterId::Permanent(i) => todo!(),
                RegisterId::Temporary(i) => *i,
            },
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

impl Cell {
    pub fn heap_address(&self) -> CellAddress {
        match self {
            Cell::StructureRef(index) => CellAddress::GlobalStack { index: *index },
            Cell::Reference(index) => CellAddress::GlobalStack { index: *index },
            _ => panic!("Unexpected call on heap address"),
        }
    }
}

#[derive(Clone, Debug)]
struct WatchCell {
    address: CellAddress,
    descriptor_id: DescriptorId,
}

impl Interpreter {
    pub fn new(
        instructions: Vec<Instruction>,
        start_instruction_index: usize,
        registers: usize,
        descriptors: Vec<TermDescriptor>,
        variables_to_watch: &[InspectionVariable],
    ) -> Self {
        Self {
            global_stack: Vec::with_capacity(1024),
            environment_stack: EnvironmentStack::new(),
            trail: Vec::with_capacity(1024),
            choice_point_stack: ChoicePointStack::new(),
            registers: vec![Cell::Undefined; registers],
            instruction_index: start_instruction_index,
            current_functor: DescriptorId(0),
            proceed_return_address: start_instruction_index,
            execution_state: ExecutionState::Normal,
            mode: Mode::Write,
            next_sub_term_address: 0,
            inspection_watch: variables_to_watch
                .iter()
                .map(|var| WatchCell {
                    descriptor_id: var.variable,
                    address: CellAddress::Register {
                        index: var.register,
                    },
                })
                .collect(),
            descriptors,
            instructions,
            inspection_set: false,
        }
    }

    fn lookup_register(&self, register: &RegisterId) -> &Cell {
        match register {
            RegisterId::Argument(index) => &self.registers[*index],
            RegisterId::Temporary(index) => &self.registers[*index],
            RegisterId::Permanent(index) => self.environment_stack.get_variable(*index),
        }
    }

    fn lookup_register_safe(&self, register: &RegisterId) -> Option<&Cell> {
        match register {
            RegisterId::Argument(index) => self.registers.get(*index),
            RegisterId::Temporary(index) => self.registers.get(*index),
            RegisterId::Permanent(index) => Some(self.environment_stack.get_variable(*index)),
        }
    }

    fn lookup_register_mut<'a>(
        environment: &'a mut EnvironmentStack,
        registers: &'a mut [Cell],
        register: RegisterId,
    ) -> &'a mut Cell {
        match register {
            RegisterId::Argument(index) => &mut registers[index],
            RegisterId::Temporary(index) => &mut registers[index],
            RegisterId::Permanent(index) => environment.get_variable_mut(index),
        }
    }

    fn lookup_address(&self, address: CellAddress) -> &Cell {
        match address {
            CellAddress::Register { index } => self.lookup_register(&index),
            CellAddress::GlobalStack { index } => &self.global_stack[index],
        }
    }

    fn lookup_address_safe(&self, address: CellAddress) -> Option<&Cell> {
        match address {
            CellAddress::Register { index } => self.lookup_register_safe(&index),
            CellAddress::GlobalStack { index } => self.global_stack.get(index),
        }
    }

    fn lookup_address_mut(&mut self, address: CellAddress) -> &mut Cell {
        match address {
            CellAddress::Register { index } => {
                Self::lookup_register_mut(&mut self.environment_stack, &mut self.registers, index)
            }
            CellAddress::GlobalStack { index } => &mut self.global_stack[index],
        }
    }

    fn try_trail(&mut self, address: CellAddress) {
        if self.choice_point_stack.is_empty() {
            return;
        }
        let choice_point_stack_address = self.choice_point_stack.get_stack_address();

        match address {
            CellAddress::GlobalStack { index } if index < choice_point_stack_address => {
                self.trail.push(address.clone());
            }
            _ => {}
        }
    }

    fn bind_address(&mut self, a: CellAddress, b: CellAddress) {
        let a_value = self.lookup_address(a);
        let b_value = self.lookup_address(b);

        let mut target = a;
        let mut value = Cell::Undefined;

        match (a, b) {
            (CellAddress::Register { .. }, _) => {
                target = b;
                value = a_value.clone();
            }
            (_, CellAddress::Register { .. }) => {
                target = a;
                value = b_value.clone();
            }
            _ => match (a_value, b_value) {
                (Cell::Reference(_), Cell::Reference(_)) if a.index_num() > b.index_num() => {
                    target = b;
                    value = Cell::Reference(a.index_num());
                }
                (Cell::Reference(_), Cell::Reference(_)) => {
                    target = a;
                    value = Cell::Reference(b.index_num());
                }
                (Cell::Reference(_), _) => {
                    target = a;
                    value = Cell::Reference(b.index_num());
                }
                (_, Cell::Reference(_)) => {
                    target = b;
                    value = Cell::Reference(a.index_num());
                }
                _ => {}
            },
        }

        self.try_trail(target);
        let target = self.lookup_address_mut(target);
        *target = value;
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

    fn deref_cell_safe(&self, address: CellAddress) -> Option<CellAddress> {
        let value = self.lookup_address_safe(address)?;
        let index = address.index_num();
        match value {
            Cell::Reference(child_address) if *child_address != index || address.is_register() => {
                self.deref_cell_safe(CellAddress::GlobalStack {
                    index: *child_address,
                })
            }
            _ => Some(address),
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
                                for i in 1..=functor_description.arity() {
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

                    self.backtrack();
                    break;
                }
                _ => {
                    self.backtrack();
                    break;
                }
            }
        }
    }

    fn unwind_trail(&mut self, range: Range<usize>) {
        for i in range {
            let stack_address = self.trail[i];
            let stack_address_raw = stack_address.index_num();
            self.global_stack[stack_address_raw] = Cell::Reference(stack_address_raw);
        }
    }

    fn backtrack(&mut self) {
        if self.choice_point_stack.is_empty() {
            self.execution_state = ExecutionState::Failure;
            return;
        }

        let next_address = self.choice_point_stack.get_next_instruction();
        self.instruction_index = next_address;
    }

    pub fn try_backtrack(&mut self) -> bool {
        if self.choice_point_stack.is_empty() || self.execution_state == ExecutionState::Failure {
            return false;
        }

        self.backtrack();
        true
    }

    pub fn step(&mut self) -> bool {
        if self.execution_state == ExecutionState::Failure {
            return false;
        }
        if self.instruction_index == self.instructions.len() {
            return false;
        }
        // TODO: Fix unneeded clone
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

                let register = Self::lookup_register_mut(
                    &mut self.environment_stack,
                    &mut self.registers,
                    *register,
                );
                *register = Cell::StructureRef(self.global_stack.len() - 1);
            }
            Instruction::SetVariable { register } => {
                self.global_stack
                    .push(Cell::Reference(self.global_stack.len()));

                let register = Self::lookup_register_mut(
                    &mut self.environment_stack,
                    &mut self.registers,
                    *register,
                );
                *register = Cell::Reference(self.global_stack.len() - 1);
            }
            Instruction::SetValue { register } => {
                let register = Self::lookup_register_mut(
                    &mut self.environment_stack,
                    &mut self.registers,
                    *register,
                );
                self.global_stack.push(register.clone());
            }
            Instruction::PutValue {
                value_register,
                argument_register,
            } => {
                let value = self.lookup_register(value_register).clone();
                let register = Self::lookup_register_mut(
                    &mut self.environment_stack,
                    &mut self.registers,
                    *argument_register,
                );
                *register = value;
            }
            Instruction::PutVariable {
                argument_register,
                variable_register,
            } => {
                let new_unbound = Cell::Reference(self.global_stack.len());
                self.global_stack.push(new_unbound.clone());

                *Self::lookup_register_mut(
                    &mut self.environment_stack,
                    &mut self.registers,
                    *argument_register,
                ) = new_unbound.clone();
                *Self::lookup_register_mut(
                    &mut self.environment_stack,
                    &mut self.registers,
                    *variable_register,
                ) = new_unbound;
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
                    Cell::Reference(_) => {
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
                            self.backtrack();
                        }
                    }
                    _ => {
                        self.backtrack();
                    }
                }
            }
            Instruction::GetVariable {
                argument_register,
                variable_register,
            } => {
                let argument_register_value = self.lookup_register(argument_register).clone();
                let variable_register_value = Self::lookup_register_mut(
                    &mut self.environment_stack,
                    &mut self.registers,
                    *variable_register,
                );
                *variable_register_value = argument_register_value;
            }
            Instruction::GetValue {
                argument_register,
                value_register,
            } => {
                self.unify(
                    CellAddress::Register {
                        index: *value_register,
                    },
                    CellAddress::Register {
                        index: *argument_register,
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
            // Control flow
            Instruction::Proceed => {
                self.instruction_index = self.proceed_return_address;
            }
            Instruction::Call { address, functor } => {
                self.proceed_return_address = self.instruction_index;
                self.instruction_index = *address;

                self.current_functor = *functor;

                // before executing the fact we collect the values of the watched registers.
                if !self.inspection_set {
                    for watch in &mut self.inspection_watch {
                        let cell = self.registers[watch.address.index_num()].heap_address();
                        watch.address = cell.clone();
                    }
                    self.inspection_set = true;
                }
            }
            Instruction::Allocate { variables } => {
                self.environment_stack
                    .push_environment(*variables, self.proceed_return_address);
            }
            Instruction::Deallocate => {
                self.instruction_index = self.environment_stack.get_continuation();
                self.environment_stack.pop_environment();
            }
            Instruction::TryMeElse { else_address } => {
                let arity = self.descriptors[self.current_functor.0].arity();
                self.choice_point_stack.push_choice_point(
                    arity,
                    self.proceed_return_address,
                    self.environment_stack.get_current_address(),
                    *else_address,
                    self.trail.len(),
                    self.global_stack.len(),
                );
                for i in 0..arity {
                    let argument = self.registers[i].clone();
                    *self.choice_point_stack.get_argument_mut(i) = argument.clone();
                }
            }
            Instruction::RetryMeElse { else_address } => {
                let arity = self.descriptors[self.current_functor.0].arity();
                for i in 0..arity {
                    let argument = &mut self.registers[i];
                    *argument = self.choice_point_stack.get_argument(i).clone();
                }
                self.environment_stack
                    .reset_to(self.choice_point_stack.get_environment_address());
                self.proceed_return_address = self.choice_point_stack.get_continuation();
                *self.choice_point_stack.get_next_instruction_mut() = *else_address;

                let trail_address = self.choice_point_stack.get_trail_address();
                self.unwind_trail(trail_address..self.trail.len());
                unsafe { self.trail.set_len(trail_address) };

                let stack_address = self.choice_point_stack.get_stack_address();
                unsafe { self.global_stack.set_len(stack_address) };
            }
            Instruction::TrustMe => {
                let arity = self.descriptors[self.current_functor.0].arity();
                for i in 0..arity {
                    let argument = &mut self.registers[i];
                    *argument = self.choice_point_stack.get_argument(i).clone();
                }

                self.environment_stack
                    .reset_to(self.choice_point_stack.get_environment_address());
                self.proceed_return_address = self.choice_point_stack.get_continuation();

                let trail_address = self.choice_point_stack.get_trail_address();
                self.unwind_trail(trail_address..self.trail.len());
                unsafe { self.trail.set_len(trail_address) };

                let stack_address = self.choice_point_stack.get_stack_address();
                unsafe { self.global_stack.set_len(stack_address) };

                self.choice_point_stack.pop_choice_point();
            }
            Instruction::NoOp => {}
        }

        true
    }

    fn inspect_variable(&self, address: CellAddress) -> InspectionView {
        let Some(deref_address) = self.deref_cell_safe(address) else {
            return InspectionView::Undefined;
        };
        let Some(cell) = self.lookup_address_safe(deref_address) else {
            return InspectionView::Undefined;
        };
        match cell {
            Cell::Reference(reference_address_index)
                if CellAddress::GlobalStack {
                    index: *reference_address_index,
                } == deref_address =>
            {
                InspectionView::UnboundVariable {
                    index: *reference_address_index,
                }
            }
            Cell::StructureRef(reference_index) => {
                self.inspect_variable(CellAddress::GlobalStack {
                    index: *reference_index,
                })
            }
            Cell::Structure(descriptor_id) => {
                let arity = self.descriptors[descriptor_id.0].arity();

                InspectionView::Structure {
                    descriptor_id: *descriptor_id,
                    arguments: (0..arity)
                        .map(|i| {
                            self.inspect_variable(CellAddress::GlobalStack {
                                index: address.index_num() + i + 1,
                            })
                        })
                        .collect(),
                }
            }
            Cell::Undefined => InspectionView::Undefined,
            _ => {
                todo!("Implement inspection for other cell types {:?}", cell)
            }
        }
    }

    pub fn inspect(&self) -> InspectionResult {
        let mut result = InspectionResult {
            variables: Vec::new(),
        };

        for variable in &self.inspection_watch {
            let view = self.inspect_variable(variable.address);
            result.variables.push((variable.descriptor_id, view));
        }

        result
    }
}

#[derive(Debug, Clone)]
pub struct InspectionVariable {
    pub variable: DescriptorId,
    pub register: RegisterId,
}

#[derive(Debug, Clone)]
pub struct InspectionResult {
    pub variables: Vec<(DescriptorId, InspectionView)>,
}

#[derive(Debug, Clone)]
pub enum InspectionView {
    UnboundVariable {
        index: usize,
    },
    Undefined,
    Structure {
        descriptor_id: DescriptorId,
        arguments: Vec<InspectionView>,
    },
}
