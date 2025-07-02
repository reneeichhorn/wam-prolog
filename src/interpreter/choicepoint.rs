use crate::interpreter::Cell;

#[derive(Clone, Debug)]
pub struct ChoicePointStack {
    raw_stack: Vec<u8>,
    last_address: usize,
    next_address: usize,
}

#[derive(Clone, Debug)]
pub struct ChoicePointHead {
    pub num_arguments: usize,
    pub continuation_address: usize,
    pub environment_address: usize,
    pub previous_address: usize,
    pub next_instruction_address: usize,
    pub trail_address: usize,
    pub stack_address: usize,
}

#[derive(Clone, Debug)]
pub struct InspectedChoicePoint {
    pub head: ChoicePointHead,
    pub arguments: Vec<Cell>,
}

impl ChoicePointStack {
    pub fn new() -> Self {
        Self {
            raw_stack: vec![0; 1024 * 10],
            last_address: 0,
            next_address: 0,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.next_address == 0
    }

    pub fn push_choice_point(
        &mut self,
        num_arguments: usize,
        continuation_address: usize,
        environment_address: usize,
        next_instruction_address: usize,
        trail_address: usize,
        stack_address: usize,
    ) {
        let head_size = std::mem::size_of::<ChoicePointHead>();
        let next_head = unsafe {
            let raw_ptr = self.raw_stack[self.next_address..self.next_address + head_size].as_ptr();
            let head = std::mem::transmute::<_, &mut ChoicePointHead>(raw_ptr);
            head
        };
        next_head.num_arguments = num_arguments;
        next_head.continuation_address = continuation_address;
        next_head.previous_address = self.last_address;
        next_head.next_instruction_address = next_instruction_address;
        next_head.trail_address = trail_address;
        next_head.stack_address = stack_address;
        next_head.environment_address = environment_address;

        self.last_address = self.next_address;
        self.next_address += head_size + num_arguments * std::mem::size_of::<Cell>();
    }

    pub fn pop_choice_point(&mut self) {
        let head_size = std::mem::size_of::<ChoicePointHead>();
        let head = unsafe {
            let raw_ptr = self.raw_stack[self.last_address..self.last_address + head_size].as_ptr();
            let head = std::mem::transmute::<_, &ChoicePointHead>(raw_ptr);
            head
        };
        self.last_address = head.previous_address;
        self.next_address -= head_size + head.num_arguments * std::mem::size_of::<Cell>();
    }

    pub fn get_argument_mut(&mut self, index: usize) -> &mut Cell {
        let head_size = std::mem::size_of::<ChoicePointHead>();
        let variable_offset = head_size + index * std::mem::size_of::<Cell>();
        let raw_ptr = self.raw_stack[self.last_address + variable_offset
            ..self.last_address + variable_offset + std::mem::size_of::<Cell>()]
            .as_ptr();
        let cell = unsafe { std::mem::transmute::<_, &mut Cell>(raw_ptr) };
        cell
    }

    pub fn get_argument(&self, index: usize) -> &Cell {
        let head_size = std::mem::size_of::<ChoicePointHead>();
        let variable_offset = head_size + index * std::mem::size_of::<Cell>();
        let raw_ptr = self.raw_stack[self.last_address + variable_offset
            ..self.last_address + variable_offset + std::mem::size_of::<Cell>()]
            .as_ptr();
        let cell = unsafe { std::mem::transmute::<_, &Cell>(raw_ptr) };
        cell
    }

    fn get_head(&self) -> &ChoicePointHead {
        let head_size = std::mem::size_of::<ChoicePointHead>();
        let raw_ptr = self.raw_stack[self.last_address..self.last_address + head_size].as_ptr();
        let head = unsafe { std::mem::transmute::<_, &ChoicePointHead>(raw_ptr) };
        head
    }

    fn get_head_mut(&mut self) -> &mut ChoicePointHead {
        let head_size = std::mem::size_of::<ChoicePointHead>();
        let raw_ptr = self.raw_stack[self.last_address..self.last_address + head_size].as_ptr();
        let head = unsafe { std::mem::transmute::<_, &mut ChoicePointHead>(raw_ptr) };
        head
    }

    pub fn get_continuation(&self) -> usize {
        let head = self.get_head();
        head.continuation_address
    }

    pub fn get_environment_address(&self) -> usize {
        let head = self.get_head();
        head.environment_address
    }

    pub fn get_trail_address(&self) -> usize {
        let head = self.get_head();
        head.trail_address
    }

    pub fn get_stack_address(&self) -> usize {
        let head = self.get_head();
        head.stack_address
    }

    pub fn get_next_instruction_mut(&mut self) -> &mut usize {
        let head = self.get_head_mut();
        &mut head.next_instruction_address
    }

    pub fn get_next_instruction(&mut self) -> usize {
        let head = self.get_head();
        head.next_instruction_address
    }

    pub fn inspect(&self) -> Vec<InspectedChoicePoint> {
        let mut environments = Vec::new();
        let mut current_offset = 0;

        if self.last_address == 0 && self.next_address == 0 {
            return environments;
        }

        loop {
            if current_offset > self.last_address {
                break;
            }

            let head_size = std::mem::size_of::<ChoicePointHead>();
            let head = unsafe {
                let raw_ptr = self.raw_stack[current_offset..current_offset + head_size].as_ptr();
                let head = std::mem::transmute::<_, &ChoicePointHead>(raw_ptr);
                head
            };
            let arguments = unsafe {
                let raw_ptr = self.raw_stack[current_offset + head_size
                    ..current_offset
                        + head_size
                        + head.num_arguments * std::mem::size_of::<Cell>()]
                    .as_ptr();
                let arguments = std::slice::from_raw_parts::<Cell>(
                    std::mem::transmute::<*const u8, *const Cell>(raw_ptr),
                    head.num_arguments,
                );
                arguments
            };
            environments.push(InspectedChoicePoint {
                head: head.clone(),
                arguments: arguments.to_vec(),
            });

            current_offset += head_size + head.num_arguments * std::mem::size_of::<Cell>();
        }

        environments
    }
}
