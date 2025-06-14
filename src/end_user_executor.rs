use anyhow::Result;

use crate::{
    compiler::{CompiledProgram, CompiledQuery, NamedReferenceStore},
    interpreter::{self, ExecutionState, Interpreter},
};

pub struct EndUserExecutor {
    program: Option<CompiledProgram>,
    query: Option<CompiledQuery>,
    interpreter: Option<Interpreter>,
}

impl EndUserExecutor {
    pub fn new() -> Self {
        EndUserExecutor {
            program: None,
            query: None,
            interpreter: None,
        }
    }

    pub fn set_program(&mut self, program: CompiledProgram) {
        self.program = Some(program);
        self.interpreter = None;
    }

    pub fn set_query(&mut self, query: CompiledQuery) {
        self.query = Some(query);
        self.interpreter = None;
    }

    fn prepare_interpreter(
        &mut self,
        reference_store: &mut NamedReferenceStore,
    ) -> Result<&mut Interpreter> {
        if self.interpreter.is_none() {
            let query = self
                .query
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("Can't execute without a query"))?;
            let program = self
                .program
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("Can't execute without a program"))?;

            let mut instructions = query.instructions.clone();
            instructions.extend(program.instructions.clone());

            let interpreter = Interpreter::new(
                instructions,
                query
                    .register_allocator
                    .register_len()
                    .max(program.register_allocator.register_len()),
                reference_store.build_functor_descriptions(),
            );

            self.interpreter = Some(interpreter);
        }

        Ok(self.interpreter.as_mut().unwrap())
    }

    pub fn execute(&mut self, reference_store: &mut NamedReferenceStore) -> Result<EndUserResult> {
        let interpreter = self.prepare_interpreter(reference_store)?;
        while interpreter.step() {}
        Ok(EndUserResult {
            success: interpreter.execution_state == ExecutionState::Normal,
        })
    }
}

pub struct EndUserResult {
    pub success: bool,
}
