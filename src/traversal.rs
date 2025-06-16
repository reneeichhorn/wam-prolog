use std::collections::{HashSet, VecDeque};

use crate::{descriptor::DescriptorIdentifier, parsing::AbstractTerm};

pub struct BreadthFirstIterator<'a> {
    queue: VecDeque<&'a AbstractTerm>,
}

impl<'a> BreadthFirstIterator<'a> {
    pub fn new(root: &'a AbstractTerm) -> Self {
        let mut queue = VecDeque::new();
        queue.push_back(root);
        Self { queue }
    }
}

impl<'a> Iterator for BreadthFirstIterator<'a> {
    type Item = &'a AbstractTerm;

    fn next(&mut self) -> Option<Self::Item> {
        let term = self.queue.pop_front()?;
        match term {
            AbstractTerm::Structure(_, sub_terms) => {
                for sub_term in sub_terms {
                    self.queue.push_back(sub_term);
                }
            }
            _ => {}
        }
        Some(term)
    }
}

pub struct PostOrderIterator<'a> {
    queue: VecDeque<&'a AbstractTerm>,
    declared: HashSet<DescriptorIdentifier>,
}

impl<'a> PostOrderIterator<'a> {
    pub fn new(root: &'a AbstractTerm) -> Self {
        let declared = HashSet::new();
        let queue = BreadthFirstIterator::new(root).collect::<VecDeque<_>>();
        Self { queue, declared }
    }
}

impl<'a> Iterator for PostOrderIterator<'a> {
    type Item = &'a AbstractTerm;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(term) = self.queue.pop_front() {
            match term {
                AbstractTerm::Structure(_, sub_terms) => {
                    let has_declared_all = sub_terms.iter().all(|sub_term| {
                        self.declared
                            .contains(&DescriptorIdentifier::from(sub_term))
                    });
                    if has_declared_all {
                        self.declared.insert(DescriptorIdentifier::from(term));
                        return Some(term);
                    } else {
                        self.queue.push_back(term);
                    }
                }
                _ => {
                    self.declared.insert(DescriptorIdentifier::from(term));
                    return Some(term);
                }
            }
        }

        None
    }
}
