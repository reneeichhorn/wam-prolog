use std::collections::HashMap;

use crate::parsing::AbstractTerm;

pub struct TermDescriptor {
    pub name: String,
    pub kind: DescriptorKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DescriptorIdentifier {
    Functor { name: String, arity: usize },
    Named(String),
}

impl From<&AbstractTerm> for DescriptorIdentifier {
    fn from(term: &AbstractTerm) -> Self {
        match term {
            AbstractTerm::Structure(name, sub_terms) => DescriptorIdentifier::Functor {
                name: name.clone(),
                arity: sub_terms.len(),
            },
            AbstractTerm::Variable(name) => DescriptorIdentifier::Named(name.clone()),
            AbstractTerm::Constant(name) => DescriptorIdentifier::Named(name.clone()),
        }
    }
}

pub enum DescriptorKind {
    Functor { arity: usize },
    Variable,
    Constant,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
pub struct DescriptorId(usize);

#[derive(Default)]
pub struct DescriptorAllocator {
    pub descriptor_map: HashMap<DescriptorIdentifier, DescriptorId>,
    pub descriptors: Vec<TermDescriptor>,
}

impl DescriptorAllocator {
    pub fn get_or_set(&mut self, term: &AbstractTerm) -> DescriptorId {
        let identifier = DescriptorIdentifier::from(term);

        if let Some(id) = self.descriptor_map.get(&identifier) {
            *id
        } else {
            let id = DescriptorId(self.descriptors.len());
            self.descriptor_map.insert(identifier, id);
            match term {
                AbstractTerm::Structure(name, sub_terms) => {
                    self.descriptors.push(TermDescriptor {
                        name: name.clone(),
                        kind: DescriptorKind::Functor {
                            arity: sub_terms.len(),
                        },
                    });
                }
                AbstractTerm::Variable(name) | AbstractTerm::Constant(name) => {
                    self.descriptors.push(TermDescriptor {
                        name: name.clone(),
                        kind: DescriptorKind::Variable,
                    });
                }
            }
            id
        }
    }
}
