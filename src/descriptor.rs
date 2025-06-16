use std::collections::HashMap;

use crate::{instructions::DescriptorId, parsing::AbstractTerm};

#[derive(Debug, Clone)]
pub struct TermDescriptor {
    pub name: String,
    pub kind: DescriptorKind,
}

impl TermDescriptor {
    pub fn new(name: String, kind: DescriptorKind) -> Self {
        TermDescriptor { name, kind }
    }

    pub fn arity(&self) -> usize {
        match &self.kind {
            DescriptorKind::Functor { arity } => *arity,
            DescriptorKind::Variable => 0,
        }
    }

    pub fn pretty_name(&self) -> String {
        match &self.kind {
            DescriptorKind::Functor { arity } => format!("{}/{}", self.name, arity),
            DescriptorKind::Variable => format!("{}(var)", self.name),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DescriptorIdentifier {
    Functor { name: String, arity: usize },
    Named { name: String },
}

impl From<&AbstractTerm> for DescriptorIdentifier {
    fn from(term: &AbstractTerm) -> Self {
        match term {
            AbstractTerm::Structure(name, sub_terms) => DescriptorIdentifier::Functor {
                name: name.clone(),
                arity: sub_terms.len(),
            },
            AbstractTerm::Variable(name) => DescriptorIdentifier::Named { name: name.clone() },
            AbstractTerm::Constant(name) => DescriptorIdentifier::Functor {
                name: name.clone(),
                arity: 0,
            },
        }
    }
}

#[derive(Debug, Clone)]
pub enum DescriptorKind {
    Functor { arity: usize },
    Variable,
}

#[derive(Default, Debug)]
pub struct DescriptorAllocator {
    pub descriptor_map: HashMap<DescriptorIdentifier, DescriptorId>,
    pub descriptors: Vec<TermDescriptor>,
}

impl DescriptorAllocator {
    pub fn get(&self, id: DescriptorId) -> &TermDescriptor {
        &self.descriptors[id.0]
    }

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
                AbstractTerm::Constant(name) => {
                    self.descriptors.push(TermDescriptor {
                        name: name.clone(),
                        kind: DescriptorKind::Functor { arity: 0 },
                    });
                }
                AbstractTerm::Variable(name) => {
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
