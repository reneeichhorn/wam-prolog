use std::collections::{HashMap, HashSet};

use crate::{
    descriptor::{DescriptorAllocator, TermDescriptor},
    instructions::{DescriptorId, Instruction, RegisterId},
    parsing::AbstractTerm,
    traversal::{BreadthFirstIterator, PostOrderIterator},
};

pub trait CompileTarget<'a> {
    type OrderedIterator: Iterator<Item = &'a AbstractTerm>;

    fn get_ordered_iterator(root: &'a AbstractTerm) -> Self::OrderedIterator;

    fn instruction_for_structure(descriptor_id: DescriptorId, register: RegisterId) -> Instruction;
    fn instruction_for_value(register: RegisterId) -> Instruction;
    fn instruction_for_variable(register: RegisterId) -> Instruction;

    fn instruction_for_debug_preamble() -> Instruction;
}

struct RegistryAllocator {
    registry_map: HashMap<DescriptorId, RegisterId>,
    registry_ordered_list: Vec<DescriptorId>,
}

impl RegistryAllocator {
    pub fn new(term: &AbstractTerm, descriptor_allocator: &mut DescriptorAllocator) -> Self {
        let mut registry_map = HashMap::new();
        let mut registry_ordered_list = Vec::new();

        let iter = BreadthFirstIterator::new(term);

        for term in iter {
            let descriptor_id = descriptor_allocator.get_or_set(term);
            if registry_map.contains_key(&descriptor_id) {
                continue;
            }
            let register_index = RegisterId(registry_map.len());
            registry_map.insert(descriptor_id, register_index);
            registry_ordered_list.push(descriptor_id);
        }

        RegistryAllocator {
            registry_map,
            registry_ordered_list,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CompileArtifact {
    pub instructions: Vec<Instruction>,
    pub registers: Vec<DescriptorId>,
}

pub fn compile<'a, T: CompileTarget<'a>>(
    root: &'a AbstractTerm,
    descriptor_allocator: &mut DescriptorAllocator,
) -> CompileArtifact {
    let mut instructions = Vec::new();
    instructions.push(T::instruction_for_debug_preamble());

    let mut processed_vars = HashSet::new();

    let registry_allocator = RegistryAllocator::new(root, descriptor_allocator);
    let iter = T::get_ordered_iterator(root);

    for term in iter {
        let descriptor_id = descriptor_allocator.get_or_set(term);
        let register_id = registry_allocator.registry_map.get(&descriptor_id).unwrap();
        match term {
            // A constant is esentially a structure with arity zero
            AbstractTerm::Constant(_) => {
                instructions.push(T::instruction_for_structure(descriptor_id, *register_id));
            }
            AbstractTerm::Structure(_, sub_terms) => {
                instructions.push(T::instruction_for_structure(descriptor_id, *register_id));

                for sub_term in sub_terms {
                    let descriptor_id = descriptor_allocator.get_or_set(sub_term);
                    let register_id = registry_allocator.registry_map.get(&descriptor_id).unwrap();

                    let sub_term_instruction = match sub_term {
                        AbstractTerm::Constant(_) | AbstractTerm::Structure(_, _) => {
                            T::instruction_for_value(*register_id)
                        }
                        AbstractTerm::Variable(_) if processed_vars.contains(register_id) => {
                            T::instruction_for_value(*register_id)
                        }
                        AbstractTerm::Variable(_) => {
                            processed_vars.insert(*register_id);
                            T::instruction_for_variable(*register_id)
                        }
                    };
                    instructions.push(sub_term_instruction);
                }
            }
            _ => {}
        }
    }

    CompileArtifact {
        instructions,
        registers: registry_allocator.registry_ordered_list,
    }
}

pub struct QueryTarget;
pub struct ProgramTarget;

impl<'a> CompileTarget<'a> for ProgramTarget {
    type OrderedIterator = BreadthFirstIterator<'a>;

    fn get_ordered_iterator(root: &'a AbstractTerm) -> Self::OrderedIterator {
        BreadthFirstIterator::new(root)
    }

    fn instruction_for_debug_preamble() -> Instruction {
        Instruction::DebugComment {
            message: Box::new("Generate code for program".to_string()),
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
    type OrderedIterator = PostOrderIterator<'a>;

    fn get_ordered_iterator(root: &'a AbstractTerm) -> Self::OrderedIterator {
        PostOrderIterator::new(root)
    }

    fn instruction_for_debug_preamble() -> Instruction {
        Instruction::DebugComment {
            message: Box::new("Generate code for query".to_string()),
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
