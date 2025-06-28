use std::collections::{HashMap, HashSet};

use ratatui::symbols::line::ROUNDED_BOTTOM_LEFT;

use crate::{
    descriptor::{DescriptorAllocator, TermDescriptor},
    instructions::{DescriptorId, Instruction, RegisterId},
    interpreter::InspectionVariable,
    parsing::{AbstractFact, AbstractProgram, AbstractRule, AbstractTerm},
    traversal::{
        AbstractTermItem, DepthFirstIterator, FactIterator, QueryIterator, generate_term_id,
    },
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

    fn instruction_for_sub_argument(register: RegisterId) -> Instruction;
}

#[derive(Debug, Clone)]
struct RegistryAllocator {
    registry_map: HashMap<RegisterIdentifier, RegisterAllocation>,
    registry_ordered_list: Vec<RegisterIdentifier>,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Copy)]
enum RegisterIdentifier {
    Variable(DescriptorId),
    NonVariable(usize),
}

#[derive(Debug, Clone)]
struct RegisterAllocation {
    register: Option<RegisterId>,
    argument_register: HashSet<RegisterId>,
}

impl RegisterAllocation {
    fn get_register_id(&self, level: usize, argument_index: usize) -> RegisterId {
        if level == 1 {
            RegisterId::Argument(argument_index)
        } else {
            self.register.unwrap()
        }
    }
}

impl RegistryAllocator {
    pub fn prepare_permanent_variables(
        rule: &AbstractRule,
        descriptor_allocator: &mut DescriptorAllocator,
    ) -> HashMap<DescriptorId, usize> {
        let mut occurance = HashMap::new();

        // We consider the head and first part of body as one group to avoid creating unnecessary permanent variables for first goal
        let mut relevant_term_groups = Vec::new();
        relevant_term_groups.push(
            DepthFirstIterator::new(&rule.head)
                .chain(
                    rule.goals
                        .iter()
                        .take(1)
                        .flat_map(|goal| DepthFirstIterator::new(goal)),
                )
                .collect::<Vec<_>>(),
        );
        relevant_term_groups.extend(
            rule.goals
                .iter()
                .skip(1)
                .map(|goal| DepthFirstIterator::new(goal).collect())
                .collect::<Vec<_>>(),
        );

        for (group_index, group) in relevant_term_groups.into_iter().enumerate() {
            for term in group {
                match term.term {
                    AbstractTerm::Variable(_) => {
                        let descriptor_id = descriptor_allocator.get_or_set(&term.term);
                        occurance
                            .entry(descriptor_id)
                            .and_modify(|set: &mut HashSet<_>| {
                                set.insert(group_index);
                            })
                            .or_insert_with(|| {
                                vec![group_index].into_iter().collect::<HashSet<_>>()
                            });
                    }
                    _ => {}
                }
            }
        }

        occurance
            .into_iter()
            .filter(|(_, occurance)| occurance.len() > 1)
            .enumerate()
            .collect::<Vec<_>>()
            .into_iter()
            .rev() // reverse just to keep allocation consistent with the book.
            .map(|(i, (id, _))| (id, i))
            .collect()
    }

    pub fn new<'a, T: CompileTarget<'a>>(
        term: &'a AbstractTerm,
        descriptor_allocator: &mut DescriptorAllocator,
        permanent_variables: &HashMap<DescriptorId, usize>,
    ) -> Self {
        let mut registry_map = HashMap::new();

        let root_arguments = match term {
            AbstractTerm::Structure(_, sub_terms) => sub_terms.len(),
            _ => 0,
        };

        let mut child_index = root_arguments;

        let iter = T::get_ordered_iterator(term);

        for term in iter {
            println!("proc essing term for allocation {:#?}", term);
            let needs_argument_register = term.level == 1;
            let needs_register = match term.term {
                AbstractTerm::Variable(_) => true,
                AbstractTerm::Constant(_) => true,
                _ => term.level != 1,
            };

            let descriptor_id = descriptor_allocator.get_or_set(term.term);
            let register_identifier = match term.term {
                AbstractTerm::Variable(_) => RegisterIdentifier::Variable(descriptor_id),
                _ => RegisterIdentifier::NonVariable(term.id),
            };

            if !registry_map.contains_key(&register_identifier) {
                registry_map.insert(
                    register_identifier,
                    RegisterAllocation {
                        register: None,
                        argument_register: HashSet::new(),
                    },
                );
            }

            let allocation = registry_map.get_mut(&register_identifier).unwrap();

            if needs_register && allocation.register.is_none() {
                allocation.register =
                    if let Some(permanent_index) = permanent_variables.get(&descriptor_id) {
                        Some(RegisterId::Permanent(*permanent_index))
                    } else {
                        child_index += 1;
                        Some(RegisterId::Temporary(child_index - 1))
                    };
            }

            if needs_argument_register
                && !allocation
                    .argument_register
                    .contains(&RegisterId::Argument(term.argument_index))
            {
                allocation
                    .argument_register
                    .insert(RegisterId::Argument(term.argument_index));
            }
        }

        let mut registry_ordered_list = Vec::new();
        for i in 0..child_index {
            let register = registry_map
                .iter()
                .find(|(_, alloc)| {
                    alloc.register == Some(RegisterId::Temporary(i))
                        || alloc.argument_register.contains(&RegisterId::Argument(i))
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

    fn get_register_raw(
        &self,
        term: &AbstractTerm,
        parent_id: usize,
        argument_index: usize,
        descriptor_allocator: &mut DescriptorAllocator,
    ) -> &RegisterAllocation {
        let identifier = match term {
            AbstractTerm::Variable(_) => {
                let descriptor_id = descriptor_allocator.get_or_set(term);
                RegisterIdentifier::Variable(descriptor_id)
            }
            _ => RegisterIdentifier::NonVariable(generate_term_id(parent_id, argument_index)),
        };
        self.registry_map.get(&identifier).unwrap()
    }

    fn get_register(
        &self,
        term: &AbstractTermItem,
        descriptor_allocator: &mut DescriptorAllocator,
    ) -> &RegisterAllocation {
        let identifier = match term.term {
            AbstractTerm::Variable(_) => {
                let descriptor_id = descriptor_allocator.get_or_set(term.term);
                RegisterIdentifier::Variable(descriptor_id)
            }
            _ => RegisterIdentifier::NonVariable(term.id),
        };
        self.registry_map.get(&identifier).unwrap()
    }

    fn get_root_argument_register(&self, index: usize) -> RegisterId {
        RegisterId::Argument(index)
    }
}

#[derive(Debug, Clone)]
pub struct CompileArtifact {
    pub instructions: Vec<Instruction>,
    pub max_registers: usize,
    pub start_instruction_index: usize,
    pub inspection_variables: Vec<InspectionVariable>,
}

#[derive(Debug, Clone)]
struct IntermediateCompileArtifact {
    pub instructions: Vec<Instruction>,
    pub register_allocator: RegistryAllocator,
}

#[derive(Debug)]
pub struct Compiler {
    instructions: Vec<Instruction>,
    fact_call_map: HashMap<DescriptorId, usize>,
    pub descriptor_allocator: DescriptorAllocator,
    max_registers: usize,
}

impl Compiler {
    pub fn new() -> Self {
        Compiler {
            instructions: Vec::new(),
            fact_call_map: HashMap::new(),
            descriptor_allocator: DescriptorAllocator::default(),
            max_registers: 0,
        }
    }

    pub fn reset(&mut self) {
        self.max_registers = 0;
        self.instructions.clear();
        self.fact_call_map.clear();
        self.descriptor_allocator = DescriptorAllocator::default();
    }

    pub fn add_program(&mut self, program: &AbstractProgram) {
        match program {
            AbstractProgram::Fact(fact) => self.add_fact(fact),
            AbstractProgram::Rule(rule) => self.add_rule(rule),
        }
    }

    pub fn add_rule(&mut self, rule: &AbstractRule) {
        let permanent_variables =
            RegistryAllocator::prepare_permanent_variables(&rule, &mut self.descriptor_allocator);

        let root_descriptor_id = self.descriptor_allocator.get_or_set(&rule.head);
        self.fact_call_map
            .insert(root_descriptor_id, self.instructions.len());

        let mut processed = HashSet::<DescriptorId>::new();

        self.instructions.push(Instruction::DebugComment {
            message: Box::new(format!("{}/{} (head)", rule.head.name(), rule.head.arity())),
        });
        self.instructions.push(Instruction::Allocate {
            variables: permanent_variables.len(),
        });
        let head = self.compile_for_target::<ProgramTarget>(
            &rule.head,
            &permanent_variables,
            &mut processed,
        );
        self.instructions.extend(head.instructions.clone());

        self.instructions.push(Instruction::DebugComment {
            message: Box::new(format!("{}/{} (body)", rule.head.name(), rule.head.arity())),
        });

        for goal in &rule.goals {
            self.instructions.push(Instruction::DebugComment {
                message: Box::new(format!("{}/{} (goal)", goal.name(), goal.arity())),
            });
            let query =
                self.compile_for_target::<QueryTarget>(&goal, &permanent_variables, &mut processed);
            self.instructions.extend(query.instructions);

            let descriptor_id = self.descriptor_allocator.get_or_set(&goal);
            let call_address = self
                .fact_call_map
                .get(&descriptor_id)
                .expect("term to exist");
            self.instructions.push(Instruction::Call {
                address: *call_address,
            });
        }

        self.instructions.push(Instruction::Deallocate);
    }

    pub fn add_fact(&mut self, fact: &AbstractFact) {
        let root_descriptor_id = self.descriptor_allocator.get_or_set(&fact.term);
        self.fact_call_map
            .insert(root_descriptor_id, self.instructions.len());

        self.instructions.push(Instruction::DebugComment {
            message: Box::new(format!("{}/{}", fact.name(), fact.arity())),
        });

        let artifact = self.compile_for_target::<ProgramTarget>(
            &fact.term,
            &Default::default(),
            &mut HashSet::new(),
        );
        self.instructions.extend(artifact.instructions.clone());

        self.instructions.push(Instruction::Proceed);
    }

    pub fn compile(&mut self, query: &AbstractProgram) -> CompileArtifact {
        let query = match query {
            AbstractProgram::Fact(fact) => &fact.term,
            _ => todo!(),
        };

        let root_descriptor_id = self.descriptor_allocator.get_or_set(query);
        let call_address = *self.fact_call_map.get(&root_descriptor_id).unwrap();

        let artifact =
            self.compile_for_target::<QueryTarget>(query, &Default::default(), &mut HashSet::new());

        let start_instruction = self.instructions.len();
        self.instructions.push(Instruction::DebugComment {
            message: Box::new(format!("{}/{} (query)", query.name(), query.arity())),
        });
        self.instructions.extend(artifact.instructions);
        self.instructions.push(Instruction::Call {
            address: call_address,
        });

        let mut unique_variables = HashSet::new();
        let inspection_variables = DepthFirstIterator::new(query)
            .filter_map(|term| match term.term {
                AbstractTerm::Variable(_) => {
                    let descriptor_id = self.descriptor_allocator.get_or_set(&term.term);
                    if unique_variables.contains(&descriptor_id) {
                        return None;
                    }
                    unique_variables.insert(descriptor_id);

                    let register = artifact
                        .register_allocator
                        .get_register(&term, &mut self.descriptor_allocator);
                    Some(InspectionVariable {
                        register: register.register.unwrap(),
                        variable: descriptor_id,
                    })
                }
                _ => None,
            })
            .collect();

        CompileArtifact {
            start_instruction_index: start_instruction,
            instructions: self.instructions.clone(),
            max_registers: self.max_registers,
            inspection_variables,
        }
    }

    fn compile_for_target<'a, T: CompileTarget<'a>>(
        &mut self,
        root: &'a AbstractTerm,
        permanent_variables: &HashMap<DescriptorId, usize>,
        processed_vars: &mut HashSet<DescriptorId>,
    ) -> IntermediateCompileArtifact {
        let mut instructions = Vec::new();

        let registry_allocator =
            RegistryAllocator::new::<T>(root, &mut self.descriptor_allocator, permanent_variables);
        let iter = T::get_ordered_iterator(root);

        println!("========= ITER =============");
        for term in iter {
            println!("Processing iter: {:#?}", term);
            println!(
                "Processing iter register: {:#?}",
                registry_allocator.registry_map
            );
            let descriptor_id = self.descriptor_allocator.get_or_set(term.term);
            let register_allocation =
                registry_allocator.get_register(&term, &mut self.descriptor_allocator);

            let mut was_processed = processed_vars.contains(&descriptor_id);

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
                    for (sub_term_index, sub_term) in sub_terms.iter().enumerate() {
                        let sub_descriptor_id = self.descriptor_allocator.get_or_set(sub_term);

                        println!(
                            "Processing inner sub term: {:#?}\nterm id: {}, index: {}",
                            sub_term_index, term.id, sub_term_index
                        );
                        let sub_register_allocation = registry_allocator.get_register_raw(
                            sub_term,
                            term.id,
                            sub_term_index,
                            &mut self.descriptor_allocator,
                        );

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
                            AbstractTerm::Structure(_, _) | AbstractTerm::Constant(_) => {
                                instructions.push(T::instruction_for_sub_argument(
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

        self.max_registers = self
            .max_registers
            .max(registry_allocator.registry_ordered_list.len());

        println!("Registers: {:#?}", registry_allocator.registry_map);

        IntermediateCompileArtifact {
            instructions,
            register_allocator: registry_allocator,
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

    fn instruction_for_sub_argument(register: RegisterId) -> Instruction {
        Instruction::UnifyVariable { register }
    }
}

impl<'a> CompileTarget<'a> for QueryTarget {
    type OrderedIterator = QueryIterator<'a>;

    fn get_ordered_iterator(root: &'a AbstractTerm) -> Self::OrderedIterator {
        QueryIterator::new(root)
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

    fn instruction_for_sub_argument(register: RegisterId) -> Instruction {
        Instruction::SetValue { register }
    }
}
