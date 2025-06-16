use anyhow::Result;

use crate::{
    compiler::CompileArtifact,
    descriptor::DescriptorAllocator,
    interpreter::{self, ExecutionState, Interpreter},
};

pub struct EndUserExecutor {
    program: Option<CompileArtifact>,
    query: Option<CompileArtifact>,
    pub interpreter: Option<Interpreter>,
}

impl EndUserExecutor {
    pub fn new() -> Self {
        EndUserExecutor {
            program: None,
            query: None,
            interpreter: None,
        }
    }

    pub fn set_program(&mut self, program: CompileArtifact) {
        self.program = Some(program);
        self.interpreter = None;
    }

    pub fn set_query(&mut self, query: CompileArtifact) {
        self.query = Some(query);
        self.interpreter = None;
    }

    fn prepare_interpreter(
        &mut self,
        descriptors: &mut DescriptorAllocator,
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
                query.registers.len().max(program.registers.len()),
                descriptors.descriptors.clone(),
            );

            self.interpreter = Some(interpreter);
        }

        Ok(self.interpreter.as_mut().unwrap())
    }

    pub fn execute(&mut self, descriptors: &mut DescriptorAllocator) -> Result<EndUserResult> {
        let interpreter = self.prepare_interpreter(descriptors)?;
        while interpreter.step() {}
        Ok(EndUserResult {
            success: interpreter.execution_state == ExecutionState::Normal,
        })
    }
}

#[derive(Debug)]
pub struct EndUserResult {
    pub success: bool,
}
