use std::collections::{HashMap, HashSet, VecDeque};

use crate::{
    instructions::{Instruction, StructureRef},
    interpreter::FunctorDescription,
    parsing::AbstractTerm,
};

#[derive(Debug, Clone, Default)]
pub struct Compiler;

pub struct CompiledQuery {
    pub instructions: Vec<Instruction>,
    pub register_allocator: RegisterAllocator,
}

pub struct CompiledProgram {
    pub register_allocator: RegisterAllocator,
    pub instructions: Vec<Instruction>,
}

trait CompileTarget {}

struct QueryTarget;
struct ProgramTarget;

#[derive(Default)]
struct RegistryAllocator2;

impl Compiler {
    pub fn compile_program(
        &mut self,
        term: &AbstractTerm,
        reference_store: &mut NamedReferenceStore,
    ) -> CompiledProgram {
        let mut register_allocator = RegisterAllocator::default();
        register_allocator.allocate(term, reference_store);

        let flattened_registers = register_allocator.flattened_registers_for_program();
        let mut instructions = Vec::new();

        instructions.push(Instruction::DebugComment {
            message: Box::new(" ---- Compiled Program ----".into()),
        });

        let mut register_queue = flattened_registers
            .iter()
            .map(|index| (true, *index))
            .collect::<VecDeque<_>>();
        let mut seen_registers = HashSet::new();

        while let Some((is_functor, register_index)) = register_queue.pop_front() {
            let register = &register_allocator.register_set[register_index];
            if !is_functor {
                // Variable or Constant
                if !seen_registers.contains(&register_index) {
                    instructions.push(Instruction::UnifyVariable {
                        register: register_index,
                    });
                } else {
                    instructions.push(Instruction::UnifyValue {
                        register: register_index,
                    });
                }

                seen_registers.insert(register_index);
            } else {
                // Functor
                for sub_register_index in register.sub_term_registers.iter().rev() {
                    register_queue.push_front((false, *sub_register_index));
                }

                instructions.push(Instruction::GetStructure {
                    structure: StructureRef(register.ref_index),
                    register: register_index,
                });

                seen_registers.insert(register_index);
            }
        }

        CompiledProgram {
            register_allocator,
            instructions,
        }
    }

    pub fn compile_query(
        &mut self,
        term: &AbstractTerm,
        reference_store: &mut NamedReferenceStore,
    ) -> CompiledQuery {
        let mut register_allocator = RegisterAllocator::default();
        register_allocator.allocate(term, reference_store);

        let flattened_registers = register_allocator.flattened_registers_for_query();

        let mut instructions = Vec::new();
        let mut register_queue = flattened_registers
            .iter()
            .map(|index| (true, *index))
            .collect::<VecDeque<_>>();
        let mut seen_registers = HashSet::new();

        instructions.push(Instruction::DebugComment {
            message: Box::new(" ---- Compiled Query ----".into()),
        });

        while let Some((is_functor, register_index)) = register_queue.pop_front() {
            let register = &register_allocator.register_set[register_index];
            if !is_functor {
                // Variable or Constant
                if !seen_registers.contains(&register_index) {
                    instructions.push(Instruction::SetVariable {
                        register: register_index,
                    });
                } else {
                    instructions.push(Instruction::SetValue {
                        register: register_index,
                    });
                }

                seen_registers.insert(register_index);
            } else {
                // Functor
                for sub_register_index in register.sub_term_registers.iter().rev() {
                    register_queue.push_front((false, *sub_register_index));
                }

                instructions.push(Instruction::PutStructure {
                    structure: StructureRef(register.ref_index),
                    register: register_index,
                });

                seen_registers.insert(register_index);
            }
        }

        CompiledQuery {
            instructions,
            register_allocator,
        }
    }
}

#[derive(Default, Debug)]
pub struct NamedReferenceStore {
    registry: HashMap<StoredName, usize>,
}

impl NamedReferenceStore {
    pub fn build_functor_descriptions(&self) -> Vec<FunctorDescription> {
        let mut descriptions = Vec::new();
        for (name, index) in &self.registry {
            let arity = match name {
                StoredName::Structure { arity, .. } => *arity,
                _ => 0,
            };
            descriptions.push(FunctorDescription { arity })
        }
        descriptions
    }

    fn get_or_insert_reference_id(&mut self, name: &StoredName) -> usize {
        let len = self.registry.len();
        let index = self.registry.entry(name.clone()).or_insert_with(|| len);
        *index
    }

    pub fn get_pretty_name(&self, index: usize) -> String {
        self.registry
            .iter()
            .find_map(|(name, idx)| {
                if *idx == index {
                    Some(match name {
                        StoredName::Variable { name } => name.clone(),
                        StoredName::Constant { name } => name.clone(),
                        StoredName::Structure { name, arity } => format!("{}/{}", name, arity),
                    })
                } else {
                    None
                }
            })
            .unwrap_or(format!("_{}", index))
    }
}

#[derive(Debug, Clone, PartialEq, Hash, Eq)]
pub enum StoredName {
    Variable { name: String },
    Constant { name: String },
    Structure { name: String, arity: usize },
}

impl Into<StoredName> for &AbstractTerm {
    fn into(self) -> StoredName {
        match self {
            AbstractTerm::Constant(name) => StoredName::Constant { name: name.clone() },
            AbstractTerm::Variable(name) => StoredName::Variable { name: name.clone() },
            AbstractTerm::Structure(name, arity) => StoredName::Structure {
                name: name.clone(),
                arity: arity.len(),
            },
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct RegisterAllocator {
    registered: HashMap<(usize, Vec<usize>), usize>,
    register_set: Vec<Register>,
}

#[derive(Debug, Clone)]
struct Register {
    ref_index: usize,
    relevant_for_flat: bool,
    sub_term_registers: Vec<usize>,
}

impl RegisterAllocator {
    pub fn register_len(&self) -> usize {
        self.register_set.len()
    }
    fn allocate(&mut self, term: &AbstractTerm, reference_store: &mut NamedReferenceStore) {
        let mut queue = VecDeque::new();
        queue.push_back((term, None));

        while let Some((current_term, parent)) = queue.pop_front() {
            let ref_index = reference_store.get_or_insert_reference_id(&current_term.into());
            let index = match current_term {
                AbstractTerm::Constant(name) | AbstractTerm::Variable(name) => {
                    let ref_args = vec![];
                    let ref_tuple = (ref_index, ref_args);
                    if let Some(index) = self.registered.get(&ref_tuple) {
                        *index
                    } else {
                        self.registered.insert(ref_tuple, self.register_set.len());
                        self.register_set.push(Register {
                            ref_index: ref_index,
                            sub_term_registers: Vec::new(),
                            relevant_for_flat: matches!(current_term, AbstractTerm::Constant(_)),
                        });
                        self.register_set.len() - 1
                    }
                }
                AbstractTerm::Structure(name, sub_terms) => {
                    let ref_args = sub_terms
                        .iter()
                        .map(|sub_term| {
                            reference_store.get_or_insert_reference_id(&sub_term.into())
                        })
                        .collect();
                    let ref_tuple = (ref_index, ref_args);

                    if let Some(index) = self.registered.get(&ref_tuple) {
                        *index
                    } else {
                        self.register_set.push(Register {
                            ref_index,
                            sub_term_registers: vec![],
                            relevant_for_flat: true,
                        });
                        let index = self.register_set.len() - 1;
                        for sub_term in sub_terms {
                            queue.push_back((sub_term, Some(index)));
                        }
                        self.registered.insert(ref_tuple, index);
                        index
                    }
                }
            };
            if let Some(parent) = parent {
                self.register_set[parent].sub_term_registers.push(index);
            }
        }
    }

    fn flattened_registers_for_query(&self) -> Vec<usize> {
        let mut flattened = Vec::new();

        let mut queue = (0..self.register_set.len()).collect::<VecDeque<_>>();
        let mut declared = HashSet::new();

        while let Some(register_index) = queue.pop_front() {
            let register = &self.register_set[register_index];
            if register.sub_term_registers.iter().all(|index| {
                declared.contains(index) || self.register_set[*index].sub_term_registers.is_empty()
            }) {
                if register.relevant_for_flat {
                    declared.insert(register_index);
                    flattened.push(register_index);
                }
            } else {
                queue.push_back(register_index);
            }
        }

        flattened
    }

    fn flattened_registers_for_program(&self) -> Vec<usize> {
        self.register_set
            .iter()
            .enumerate()
            .filter_map(|(i, register)| {
                if register.relevant_for_flat {
                    Some(i)
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn pretty_term(&self, store: &NamedReferenceStore, index: usize) -> String {
        let name = self
            .registered
            .iter()
            .find_map(|(ref_index, idx)| {
                if *idx == index {
                    Some(store.get_pretty_name(ref_index.0))
                } else {
                    None
                }
            })
            .unwrap_or(format!("_{}", index));
        return name;
    }

    fn pretty_register(&self, store: &NamedReferenceStore, index: usize) -> String {
        let name = self
            .registered
            .iter()
            .find_map(|(ref_index, idx)| {
                if *idx == index {
                    Some(store.get_pretty_name(ref_index.0))
                } else {
                    None
                }
            })
            .unwrap_or(format!("_{}", index));
        let register = &self.register_set[index];

        if register.sub_term_registers.is_empty() {
            format!("X{} = {}", index + 1, name)
        } else {
            format!(
                "X{}: {}({})",
                index + 1,
                name,
                register
                    .sub_term_registers
                    .iter()
                    .map(|&idx| format!("X{}", idx + 1))
                    .collect::<Vec<_>>()
                    .join(",")
            )
        }
    }

    pub fn pretty_print_registers_flattened_query(
        &self,
        store: &NamedReferenceStore,
    ) -> Vec<String> {
        self.flattened_registers_for_query()
            .iter()
            .map(|&index| self.pretty_register(store, index))
            .collect::<Vec<_>>()
    }

    pub fn pretty_print_registers_flattened_program(
        &self,
        store: &NamedReferenceStore,
    ) -> Vec<String> {
        self.flattened_registers_for_program()
            .iter()
            .map(|&index| self.pretty_register(store, index))
            .collect::<Vec<_>>()
    }

    pub fn pretty_print_registers_all(&self, store: &NamedReferenceStore) -> Vec<String> {
        (0..self.register_set.len())
            .map(|index| self.pretty_register(store, index))
            .collect::<Vec<_>>()
    }

    pub fn pretty_print_registers(
        &self,
        store: &NamedReferenceStore,
        registers: &[usize],
    ) -> Vec<String> {
        registers
            .iter()
            .map(|&index| self.pretty_register(store, index))
            .collect::<Vec<_>>()
    }
}
