use std::collections::{HashSet, VecDeque};

use crate::{descriptor::DescriptorIdentifier, parsing::AbstractTerm};

#[derive(Debug)]
pub struct AbstractTermItem<'a> {
    pub term: &'a AbstractTerm,
    pub level: usize,
    pub argument_index: usize,
}

// Breadth-first iterator without root node
pub struct FactIterator<'a> {
    queue: VecDeque<AbstractTermItem<'a>>,
}

impl<'a> FactIterator<'a> {
    pub fn new(root: &'a AbstractTerm) -> Self {
        let mut queue = VecDeque::new();
        queue.push_back(AbstractTermItem {
            term: root,
            level: 0,
            argument_index: 0,
        });
        Self { queue }
    }
}

impl<'a> Iterator for FactIterator<'a> {
    type Item = AbstractTermItem<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let term = self.queue.pop_front()?;
        match term.term {
            AbstractTerm::Structure(_, sub_terms) => {
                for (argument_index, sub_term) in sub_terms.iter().enumerate() {
                    self.queue.push_back(AbstractTermItem {
                        term: sub_term,
                        level: term.level + 1,
                        argument_index,
                    });
                }
                if term.level == 0 {
                    return self.next();
                }
            }
            _ => {}
        }
        Some(term)
    }
}

// Post-order iterator
pub struct QueryIterator<'a> {
    queue: VecDeque<AbstractTermItem<'a>>,
    declared: HashSet<DescriptorIdentifier>,
}

impl<'a> QueryIterator<'a> {
    pub fn new(root: &'a AbstractTerm) -> Self {
        let declared = HashSet::new();
        let queue = FactIterator::new(root).collect::<VecDeque<_>>();
        println!("Queue: {:?}", queue);
        Self { queue, declared }
    }
}

impl<'a> Iterator for QueryIterator<'a> {
    type Item = AbstractTermItem<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(term) = self.queue.pop_front() {
            match term.term {
                AbstractTerm::Structure(_, sub_terms) => {
                    let has_declared_all = sub_terms.iter().all(|sub_term| {
                        self.declared
                            .contains(&DescriptorIdentifier::from(sub_term))
                    });
                    if has_declared_all {
                        self.declared.insert(DescriptorIdentifier::from(term.term));
                        return Some(term);
                    } else {
                        self.queue.push_back(term);
                    }
                }
                _ => {
                    self.declared.insert(DescriptorIdentifier::from(term.term));
                    return Some(term);
                }
            }
        }

        None
    }
}

pub struct DepthFirstIterator<'a> {
    stack: Vec<AbstractTermItem<'a>>,
}

impl<'a> DepthFirstIterator<'a> {
    pub fn new(root: &'a AbstractTerm) -> Self {
        let mut stack = Vec::new();
        stack.push(AbstractTermItem {
            term: root,
            level: 0,
            argument_index: 0,
        });
        Self { stack }
    }
}

impl<'a> Iterator for DepthFirstIterator<'a> {
    type Item = AbstractTermItem<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let term = self.stack.pop()?;
        match term.term {
            AbstractTerm::Structure(_, sub_terms) => {
                // Push children in reverse order so they're processed in correct order
                for (argument_index, sub_term) in sub_terms.iter().enumerate().rev() {
                    self.stack.push(AbstractTermItem {
                        term: sub_term,
                        level: term.level + 1,
                        argument_index,
                    });
                }
                if term.level == 0 {
                    return self.next();
                }
            }
            _ => {}
        }
        Some(term)
    }
}
