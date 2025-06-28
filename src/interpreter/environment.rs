use crate::interpreter::Cell;

#[derive(Clone, Debug)]
pub struct EnvironmentStack {
    raw_stack: Vec<u8>,
    last_environment_address: usize,
    next_environment_address: usize,
}

#[derive(Clone, Debug)]
pub struct EnvironmentHead {
    pub num_variables: usize,
    pub continuation_address: usize,
    pub previous_environment_address: usize,
}

#[derive(Clone, Debug)]
pub struct InspectedEnvironment {
    pub head: EnvironmentHead,
    pub variables: Vec<Cell>,
}

impl EnvironmentStack {
    pub fn new() -> Self {
        Self {
            raw_stack: vec![0; 1024 * 10],
            last_environment_address: 0,
            next_environment_address: 0,
        }
    }

    pub fn push_environment(&mut self, num_variables: usize, continuation_address: usize) {
        let head_size = std::mem::size_of::<EnvironmentHead>();
        let next_head = unsafe {
            let raw_ptr = self.raw_stack
                [self.next_environment_address..self.next_environment_address + head_size]
                .as_ptr();
            let head = std::mem::transmute::<_, &mut EnvironmentHead>(raw_ptr);
            head
        };
        next_head.num_variables = num_variables;
        next_head.continuation_address = continuation_address;
        next_head.previous_environment_address = self.last_environment_address;

        self.last_environment_address = self.next_environment_address;
        self.next_environment_address += head_size + num_variables * std::mem::size_of::<Cell>();
    }

    pub fn pop_environment(&mut self) {
        let head_size = std::mem::size_of::<EnvironmentHead>();
        let head = unsafe {
            let raw_ptr = self.raw_stack
                [self.last_environment_address..self.last_environment_address + head_size]
                .as_ptr();
            let head = std::mem::transmute::<_, &EnvironmentHead>(raw_ptr);
            head
        };
        self.last_environment_address = head.previous_environment_address;
        self.next_environment_address -=
            head_size + head.num_variables * std::mem::size_of::<Cell>();
    }

    pub fn get_variable_mut(&mut self, index: usize) -> &mut Cell {
        let head_size = std::mem::size_of::<EnvironmentHead>();
        let variable_offset = head_size + index * std::mem::size_of::<Cell>();
        let raw_ptr = self.raw_stack[self.last_environment_address + variable_offset
            ..self.last_environment_address + variable_offset + std::mem::size_of::<Cell>()]
            .as_ptr();
        let cell = unsafe { std::mem::transmute::<_, &mut Cell>(raw_ptr) };
        cell
    }

    pub fn get_variable(&self, index: usize) -> &Cell {
        let head_size = std::mem::size_of::<EnvironmentHead>();
        let variable_offset = head_size + index * std::mem::size_of::<Cell>();
        let raw_ptr = self.raw_stack[self.last_environment_address + variable_offset
            ..self.last_environment_address + variable_offset + std::mem::size_of::<Cell>()]
            .as_ptr();
        let cell = unsafe { std::mem::transmute::<_, &Cell>(raw_ptr) };
        cell
    }

    pub fn get_continuation(&self) -> usize {
        let head_size = std::mem::size_of::<EnvironmentHead>();
        let head = unsafe {
            let raw_ptr = self.raw_stack
                [self.last_environment_address..self.last_environment_address + head_size]
                .as_ptr();
            let head = std::mem::transmute::<_, &EnvironmentHead>(raw_ptr);
            head
        };
        head.continuation_address
    }

    pub fn inspect(&self) -> Vec<InspectedEnvironment> {
        let mut environments = Vec::new();
        let mut current_offset = 0;

        if self.last_environment_address == 0 && self.next_environment_address == 0 {
            return environments;
        }

        loop {
            if current_offset > self.last_environment_address {
                break;
            }

            let head_size = std::mem::size_of::<EnvironmentHead>();
            let head = unsafe {
                let raw_ptr = self.raw_stack[current_offset..current_offset + head_size].as_ptr();
                let head = std::mem::transmute::<_, &EnvironmentHead>(raw_ptr);
                head
            };
            let variables = unsafe {
                let raw_ptr = self.raw_stack[current_offset + head_size
                    ..current_offset
                        + head_size
                        + head.num_variables * std::mem::size_of::<Cell>()]
                    .as_ptr();
                let variables = std::slice::from_raw_parts::<Cell>(
                    std::mem::transmute::<*const u8, *const Cell>(raw_ptr),
                    head.num_variables,
                );
                variables
            };
            environments.push(InspectedEnvironment {
                head: head.clone(),
                variables: variables.to_vec(),
            });

            current_offset += head_size + head.num_variables * std::mem::size_of::<Cell>();
        }

        environments
    }
}
