use std::collections::{HashMap, HashSet};

use ratatui::symbols::line::ROUNDED_BOTTOM_LEFT;

use crate::{
    descriptor::{DescriptorAllocator, TermDescriptor},
    instructions::{DescriptorId, Instruction, RegisterId},
    parsing::AbstractTerm,
    traversal::{AbstractTermItem, DepthFirstIterator, FactIterator, QueryIterator},
};

pub trait CompileTarget<'a> {
    type OrderedIterator: Iterator<Item = AbstractTermItem<'a>>;

    fn get_ordered_iterator(root: &'a AbstractTerm) -> Self::OrderedIterator;

    fn instruction_for_structure(descriptor_id: DescriptorId, register: RegisterId) -> Instruction;

    fn instruction_for_value_argument(argument: RegisterId, value: RegisterId) -> Instruction;

    fn instruction_for_variable_argument(argument: RegisterId, variable: RegisterId)
    -> Instruction;

    fn instruction_for_value(register: RegisterId) -> Instruction;

    fn instruction_for_variable(register: RegisterId) -> Instruction;

    fn instruction_for_debug_preamble() -> Instruction;
}

struct RegistryAllocator {
    registry_map: HashMap<DescriptorId, RegisterAllocation>,
    registry_ordered_list: Vec<DescriptorId>,
}

#[derive(Debug)]
struct RegisterAllocation {
    register: Option<RegisterId>,
    argument_register: HashSet<RegisterId>,
}

impl RegisterAllocation {
    fn get_register_id(&self, level: usize, argument_index: usize) -> RegisterId {
        if level == 1 {
            RegisterId(argument_index)
        } else {
            self.register.unwrap()
        }
    }
}

impl RegistryAllocator {
    pub fn new(term: &AbstractTerm, descriptor_allocator: &mut DescriptorAllocator) -> Self {
        let mut registry_map = HashMap::new();

        let root_arguments = match term {
            AbstractTerm::Structure(_, sub_terms) => sub_terms.len(),
            _ => 0,
        };

        let mut child_index = root_arguments;

        let iter = DepthFirstIterator::new(term);

        for term in iter {
            let needs_argument_register = term.level == 1;
            let needs_register = match term.term {
                AbstractTerm::Variable(_) => true,
                AbstractTerm::Constant(_) => true,
                _ => term.level != 1,
            };

            let descriptor_id = descriptor_allocator.get_or_set(term.term);
            if !registry_map.contains_key(&descriptor_id) {
                registry_map.insert(
                    descriptor_id,
                    RegisterAllocation {
                        register: None,
                        argument_register: HashSet::new(),
                    },
                );
            }

            let allocation = registry_map.get_mut(&descriptor_id).unwrap();

            if needs_register && allocation.register.is_none() {
                child_index += 1;
                allocation.register = Some(RegisterId(child_index - 1));
            }

            if needs_argument_register
                && !allocation
                    .argument_register
                    .contains(&RegisterId(term.argument_index))
            {
                allocation
                    .argument_register
                    .insert(RegisterId(term.argument_index));
            }
        }

        let mut registry_ordered_list = Vec::new();
        for i in 0..child_index {
            let register = registry_map
                .iter()
                .find(|(_, alloc)| {
                    alloc.register == Some(RegisterId(i))
                        || alloc.argument_register.contains(&RegisterId(i))
                })
                .unwrap()
                .0;
            registry_ordered_list.push(*register);
        }

        RegistryAllocator {
            registry_map,
            registry_ordered_list,
        }
    }

    fn get_root_argument_register(&self, index: usize) -> RegisterId {
        RegisterId(index)
    }
}

#[derive(Debug, Clone)]
pub struct CompileArtifact {
    pub instructions: Vec<Instruction>,
    pub registers: Vec<DescriptorId>,
}

#[derive(Debug)]
pub struct Compiler {
    instructions: Vec<Instruction>,
    fact_call_map: HashMap<DescriptorId, usize>,
    pub descriptor_allocator: DescriptorAllocator,
}

impl Compiler {
    pub fn new() -> Self {
        Compiler {
            instructions: Vec::new(),
            fact_call_map: HashMap::new(),
            descriptor_allocator: DescriptorAllocator::default(),
        }
    }

    pub fn reset(&mut self) {
        self.instructions.clear();
        self.fact_call_map.clear();
        self.descriptor_allocator = DescriptorAllocator::default();
    }

    pub fn add_fact(&mut self, fact: &AbstractTerm) -> CompileArtifact {
        let root_descriptor_id = self.descriptor_allocator.get_or_set(fact);
        self.fact_call_map
            .insert(root_descriptor_id, self.instructions.len());

        self.instructions.push(Instruction::DebugComment {
            message: Box::new(format!("{}/{}", fact.name(), fact.arity())),
        });

        let artifact = self.compile_for_target::<ProgramTarget>(fact);
        self.instructions.extend(artifact.instructions.clone());

        self.instructions.push(Instruction::Proceed);

        artifact
    }

    pub fn compile(&mut self, query: &AbstractTerm) -> CompileArtifact {
        let root_descriptor_id = self.descriptor_allocator.get_or_set(query);
        let call_address = *self.fact_call_map.get(&root_descriptor_id).unwrap();

        let mut artifact = self.compile_for_target::<QueryTarget>(query);

        artifact.instructions.push(Instruction::Call {
            address: call_address + artifact.instructions.len() + 1,
        });
        self.instructions.splice(0..0, artifact.instructions);

        CompileArtifact {
            instructions: self.instructions.clone(),
            registers: artifact.registers,
        }
    }

    fn compile_for_target<'a, T: CompileTarget<'a>>(
        &mut self,
        root: &'a AbstractTerm,
    ) -> CompileArtifact {
        let mut instructions = Vec::new();
        instructions.push(T::instruction_for_debug_preamble());

        let mut processed_vars = HashSet::new();

        let registry_allocator = RegistryAllocator::new(root, &mut self.descriptor_allocator);
        let iter = T::get_ordered_iterator(root);

        for term in iter {
            let descriptor_id = self.descriptor_allocator.get_or_set(term.term);
            let register_allocation = registry_allocator.registry_map.get(&descriptor_id).unwrap();

            let mut was_processed = processed_vars.contains(&descriptor_id);

            instructions.push(Instruction::DebugComment {
                message: Box::new(format!("{:?}", term.term)),
            });

            match term.term {
                AbstractTerm::Variable(_) if was_processed && term.level == 1 => {
                    instructions.push(T::instruction_for_value_argument(
                        registry_allocator.get_root_argument_register(term.argument_index),
                        register_allocation.register.unwrap(),
                    ));
                    was_processed = true;
                }
                AbstractTerm::Variable(_) if term.level == 1 => {
                    instructions.push(T::instruction_for_variable_argument(
                        registry_allocator.get_root_argument_register(term.argument_index),
                        register_allocation.register.unwrap(),
                    ));
                    was_processed = true;
                }
                AbstractTerm::Constant(_) => {
                    instructions.push(T::instruction_for_structure(
                        descriptor_id,
                        register_allocation.get_register_id(term.level, term.argument_index),
                    ));
                    was_processed = true;
                }
                AbstractTerm::Structure(_, sub_terms) => {
                    instructions.push(T::instruction_for_structure(
                        descriptor_id,
                        register_allocation.get_register_id(term.level, term.argument_index),
                    ));
                    for sub_term in sub_terms {
                        let sub_descriptor_id = self.descriptor_allocator.get_or_set(sub_term);

                        let sub_register_allocation = registry_allocator
                            .registry_map
                            .get(&sub_descriptor_id)
                            .unwrap();

                        let was_processed = !processed_vars.insert(sub_descriptor_id);

                        match sub_term {
                            AbstractTerm::Variable(_) if was_processed => {
                                instructions.push(T::instruction_for_value(
                                    sub_register_allocation.register.unwrap(),
                                ));
                            }
                            AbstractTerm::Variable(_) => {
                                instructions.push(T::instruction_for_variable(
                                    sub_register_allocation.register.unwrap(),
                                ));
                            }
                            AbstractTerm::Structure(_, _) => {
                                instructions.push(T::instruction_for_variable(
                                    sub_register_allocation.register.unwrap(),
                                ));
                            }
                            AbstractTerm::Constant(_) => {
                                instructions.push(T::instruction_for_value(
                                    sub_register_allocation.register.unwrap(),
                                ));
                            }
                        }
                    }
                }
                _ => {}
            }
            if was_processed {
                processed_vars.insert(descriptor_id);
            }
        }

        CompileArtifact {
            instructions,
            registers: registry_allocator.registry_ordered_list,
        }
    }
}

pub struct QueryTarget;
pub struct ProgramTarget;

impl<'a> CompileTarget<'a> for ProgramTarget {
    type OrderedIterator = FactIterator<'a>;

    fn get_ordered_iterator(root: &'a AbstractTerm) -> Self::OrderedIterator {
        FactIterator::new(root)
    }

    fn instruction_for_debug_preamble() -> Instruction {
        Instruction::DebugComment {
            message: Box::new("Generate code for program".to_string()),
        }
    }

    fn instruction_for_value_argument(argument: RegisterId, value: RegisterId) -> Instruction {
        Instruction::GetValue {
            argument_register: argument,
            value_register: value,
        }
    }

    fn instruction_for_variable_argument(
        argument: RegisterId,
        variable: RegisterId,
    ) -> Instruction {
        Instruction::GetVariable {
            argument_register: argument,
            variable_register: variable,
        }
    }

    fn instruction_for_value(register: RegisterId) -> Instruction {
        Instruction::UnifyValue { register }
    }

    fn instruction_for_variable(register: RegisterId) -> Instruction {
        Instruction::UnifyVariable { register }
    }

    fn instruction_for_structure(descriptor_id: DescriptorId, register: RegisterId) -> Instruction {
        Instruction::GetStructure {
            structure: descriptor_id,
            register,
        }
    }
}

impl<'a> CompileTarget<'a> for QueryTarget {
    type OrderedIterator = QueryIterator<'a>;

    fn get_ordered_iterator(root: &'a AbstractTerm) -> Self::OrderedIterator {
        QueryIterator::new(root)
    }

    fn instruction_for_debug_preamble() -> Instruction {
        Instruction::DebugComment {
            message: Box::new("Generate code for query".to_string()),
        }
    }

    fn instruction_for_variable_argument(
        argument: RegisterId,
        variable: RegisterId,
    ) -> Instruction {
        Instruction::PutVariable {
            argument_register: argument,
            variable_register: variable,
        }
    }

    fn instruction_for_value_argument(argument: RegisterId, value: RegisterId) -> Instruction {
        Instruction::PutValue {
            argument_register: argument,
            value_register: value,
        }
    }

    fn instruction_for_value(register: RegisterId) -> Instruction {
        Instruction::SetValue { register }
    }

    fn instruction_for_variable(register: RegisterId) -> Instruction {
        Instruction::SetVariable { register }
    }

    fn instruction_for_structure(descriptor_id: DescriptorId, register: RegisterId) -> Instruction {
        Instruction::PutStructure {
            structure: descriptor_id,
            register,
        }
    }
}
