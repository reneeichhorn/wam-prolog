use std::collections::VecDeque;

use crate::parsing::AbstractTerm;

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
