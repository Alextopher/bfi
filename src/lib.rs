mod interpreter;

#[cfg(test)]
mod tests;

use Error::*;

pub use bfc_ir::{optimize, parse, OptimisationsFlags};
pub use interpreter::{Interpreter, RunTimeError};

pub enum Error {
    ParseError(bfc_ir::ParseError),
    RunTimeError((Vec<u8>, interpreter::RunTimeError)),
}

pub enum TestResults {
    OutputsDontMatchInputs,
    ParseError(bfc_ir::ParseError),
    Results(Vec<TestResult>),
}

pub enum TestResult {
    Ok,
    RunTimeError((Vec<u8>, interpreter::RunTimeError)),
    UnexpectedOutput { expected: Vec<u8>, output: Vec<u8> },
}

/// Executes a Brainfuck program to completion
pub fn execute(program: &str, input: &[u8], max_iterations: u64) -> Result<Vec<u8>, Error> {
    let mut instructions = bfc_ir::parse(program).map_err(ParseError)?;

    let flags = OptimisationsFlags::all();
    (instructions, _) = bfc_ir::optimize(instructions, flags);

    let interpreter = Interpreter::new(instructions, max_iterations);

    let results = interpreter.run(input).map_err(Error::RunTimeError)?;

    Ok(results)
}

/// Test a Brainfuck program
pub fn test_blocking(
    program: &str,
    inputs: &[u8],
    outputs: &[u8],
    max_iterations: u64,
) -> TestResults {
    tests_blocking(program, &[inputs], &[outputs], max_iterations)
}

pub fn tests_blocking(
    program: &str,
    inputs: &[&[u8]],
    outputs: &[&[u8]],
    max_iterations: u64,
) -> TestResults {
    if inputs.len() != outputs.len() {
        return TestResults::OutputsDontMatchInputs;
    }

    let instructions = match bfc_ir::parse(program) {
        Ok(instructions) => {
            let (inst, _) = bfc_ir::optimize(instructions, OptimisationsFlags::all());
            inst
        }
        Err(err) => return TestResults::ParseError(err),
    };

    let interpreter = Interpreter::new(instructions, max_iterations);
    let mut results: Vec<TestResult> = Vec::with_capacity(inputs.len());

    for (input, expected) in inputs.iter().zip(outputs) {
        match interpreter.run(input) {
            Ok(output) => {
                if *expected != output {
                    results.push(TestResult::UnexpectedOutput {
                        expected: Vec::from(*expected),
                        output,
                    });
                } else {
                    results.push(TestResult::Ok);
                }
            }
            Err(e) => results.push(TestResult::RunTimeError(e)),
        }
    }

    TestResults::Results(results)
}
