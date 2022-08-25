mod interpreter;

#[cfg(test)]
mod tests;

use std::{num::Wrapping, thread};

pub use bfc_ir::{optimize, parse, OptimisationsFlags};
pub use interpreter::{Interpreter, RunTimeError};

use Error::*;

pub enum Error {
    ParseError(bfc_ir::ParseError),
    RunTimeError(interpreter::RunTimeError),
}

/// Executes a brainfuck program a completion
pub fn execute(
    program: &str,
    input: &[u8],
    max_iterations: u64,
    optimize: bool,
) -> Result<Vec<u8>, Error> {
    let mut instructions = bfc_ir::parse(program).map_err(ParseError)?;

    if optimize {
        let flags = OptimisationsFlags::all();
        (instructions, _) = bfc_ir::optimize(instructions, flags);
    }

    let (tx, rx, interpreter) = Interpreter::new(&instructions, max_iterations);

    for b in input {
        tx.send(Wrapping(*b)).unwrap();
    }

    // Return results into a vector
    let results = thread::scope(move |_| {
        let mut results = Vec::new();
        while let Ok(b) = rx.recv() {
            results.push(b.0)
        }
        results
    });

    interpreter.run().map_err(RunTimeError)?;

    Ok(results)
}
