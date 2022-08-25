use std::{
    cmp::Ordering,
    num::Wrapping,
    sync::mpsc::{channel, Receiver, RecvError, SendError, Sender},
};

use bfc_ir::AstNode;

#[derive(Debug)]
pub enum RunTimeError {
    OutOfBoundsLeft,
    OutOfBoundsRight,
    RecvError(RecvError),
    SendError(SendError<Wrapping<u8>>),
    MaxIterationsExceeded,
}

/// Interpreter thats recieves inputs and sends outputs down channels
pub struct Interpreter<'a> {
    instructions: &'a [AstNode],
    memory: Vec<Wrapping<u8>>,
    memory_pointer: isize,
    max_iterations: u64,
    iterations: u64,

    inputs: Receiver<Wrapping<u8>>,
    outputs: Sender<Wrapping<u8>>,
}

impl<'a> Interpreter<'a> {
    pub fn new(
        instructions: &'a [AstNode],
        max_iterations: u64,
    ) -> (Sender<Wrapping<u8>>, Receiver<Wrapping<u8>>, Self) {
        // Create two channels to handle inputs and outputs
        let (input_tx, input_rx) = channel::<Wrapping<u8>>();
        let (output_tx, output_rx) = channel::<Wrapping<u8>>();

        (
            input_tx,
            output_rx,
            Self {
                instructions,
                memory: vec![Wrapping(0); 30000],
                memory_pointer: 0,
                max_iterations,
                iterations: 0,
                inputs: input_rx,
                outputs: output_tx,
            },
        )
    }

    /// Run the interpreter in a new thread
    pub fn run(mut self) -> Result<(), RunTimeError> {
        self.run_body(self.instructions)
    }

    fn run_body(&mut self, body: &[AstNode]) -> Result<(), RunTimeError> {
        for instruction in body {
            self.iterations += 1;
            if self.iterations > self.max_iterations {
                return Err(RunTimeError::MaxIterationsExceeded);
            }

            match instruction {
                AstNode::Increment { amount, offset, .. } => {
                    let index = self
                        .memory_pointer
                        .checked_add(*offset)
                        .ok_or(RunTimeError::OutOfBoundsRight)?;

                    // Convert isize to usize
                    let index = match index.cmp(&0) {
                        Ordering::Greater => index as usize,
                        Ordering::Equal => 0,
                        Ordering::Less => return Err(RunTimeError::OutOfBoundsLeft),
                    };

                    // Check if the index is out of bounds
                    if index >= self.memory.len() {
                        return Err(RunTimeError::OutOfBoundsRight);
                    }

                    match amount.0.cmp(&0) {
                        Ordering::Less => self.memory[index] -= amount.0.unsigned_abs(),
                        Ordering::Equal => {}
                        Ordering::Greater => self.memory[index] += amount.0.unsigned_abs(),
                    }
                }
                AstNode::PointerIncrement { amount, .. } => {
                    self.memory_pointer += amount;

                    if self.memory_pointer < 0 {
                        return Err(RunTimeError::OutOfBoundsLeft);
                    }
                    if self.memory_pointer.unsigned_abs() > self.memory.len() {
                        return Err(RunTimeError::OutOfBoundsRight);
                    }
                }
                AstNode::Read { .. } => {
                    self.memory[self.memory_pointer as usize] =
                        self.inputs.recv().map_err(RunTimeError::RecvError)?
                }
                AstNode::Write { .. } => {
                    self.outputs
                        .send(self.memory[self.memory_pointer as usize])
                        .map_err(RunTimeError::SendError)?;
                }
                AstNode::Loop { body, .. } => {
                    while self.memory[self.memory_pointer as usize] != Wrapping(0) {
                        self.run_body(body)?;
                    }
                }
                AstNode::Set { amount, offset, .. } => {
                    let index = self
                        .memory_pointer
                        .checked_add(*offset)
                        .ok_or(RunTimeError::OutOfBoundsRight)?;

                    // Convert isize to usize
                    let index = match index.cmp(&0) {
                        Ordering::Greater => index as usize,
                        Ordering::Equal => 0,
                        Ordering::Less => return Err(RunTimeError::OutOfBoundsLeft),
                    };

                    // Check if the index is out of bounds
                    if index >= self.memory.len() {
                        return Err(RunTimeError::OutOfBoundsRight);
                    }

                    // Convert the i8 to Wrapped u8
                    self.memory[index] = match amount.0.cmp(&0) {
                        Ordering::Less => -Wrapping(amount.0.unsigned_abs()),
                        Ordering::Equal => Wrapping(0),
                        Ordering::Greater => Wrapping(amount.0.unsigned_abs()),
                    }
                }
                AstNode::MultiplyMove { changes, .. } => {
                    let current = self.memory[self.memory_pointer as usize];

                    if current != Wrapping(0) {
                        for (offset, factor) in changes.iter() {
                            let index = self
                                .memory_pointer
                                .checked_add(*offset)
                                .ok_or(RunTimeError::OutOfBoundsRight)?;

                            // Convert isize to usize
                            let index = match index.cmp(&0) {
                                Ordering::Greater => index as usize,
                                Ordering::Equal => 0,
                                Ordering::Less => return Err(RunTimeError::OutOfBoundsLeft),
                            };

                            // Check if the index is out of bounds
                            if index >= self.memory.len() {
                                return Err(RunTimeError::OutOfBoundsRight);
                            }

                            self.memory[index] += current
                                * match factor.0.cmp(&0) {
                                    Ordering::Less => -Wrapping(factor.0.unsigned_abs()),
                                    Ordering::Equal => Wrapping(0),
                                    Ordering::Greater => Wrapping(factor.0.unsigned_abs()),
                                };
                        }

                        self.memory[self.memory_pointer as usize] = Wrapping(0);
                    }
                }
            }
        }

        Ok(())
    }
}
