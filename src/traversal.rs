use std::collections::{HashSet, VecDeque};

use crate::parsing::AbstractTerm;

#[derive(Debug, Clone)]
pub struct AbstractTermItem<'a> {
    pub term: &'a AbstractTerm,
    pub level: usize,
    pub argument_index: usize,
    pub id: usize,
}

pub fn generate_term_id(parent: usize, argument_index: usize) -> usize {
    parent << 4 | argument_index
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
            id: 1,
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
                        id: generate_term_id(term.id, argument_index),
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

// Post-order iterator, returns first level in order
pub struct QueryIterator<'a> {
    queue: VecDeque<AbstractTermItem<'a>>,
    declared: HashSet<usize>,
}

impl<'a> QueryIterator<'a> {
    pub fn new(root: &'a AbstractTerm) -> Self {
        let declared = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(AbstractTermItem {
            term: root,
            level: 0,
            id: 1,
            argument_index: 0,
        });
        Self { queue, declared }
    }
}

impl<'a> Iterator for QueryIterator<'a> {
    type Item = AbstractTermItem<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(term) = self.queue.pop_front() {
            match term.term {
                AbstractTerm::Structure(_, sub_terms) => {
                    let has_declared_all = sub_terms.iter().enumerate().all(|(index, sub_term)| {
                        self.declared.contains(&generate_term_id(term.id, index))
                    });
                    if has_declared_all {
                        self.declared.insert(term.id);
                        if term.level > 0 {
                            return Some(term);
                        }
                    } else {
                        self.queue.push_front(term.clone());
                        for (argument_index, sub_term) in sub_terms.iter().enumerate().rev() {
                            self.queue.push_front(AbstractTermItem {
                                term: sub_term,
                                level: term.level + 1,
                                id: generate_term_id(term.id, argument_index),
                                argument_index,
                            });
                        }
                    }
                }
                _ => {
                    self.declared.insert(term.id);
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
            id: 1,
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
                        id: generate_term_id(term.id, argument_index),
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
