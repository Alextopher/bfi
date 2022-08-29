use std::{
    cmp::Ordering,
    num::Wrapping,
    sync::{
        mpsc::{channel, Receiver, Sender},
        Arc,
    },
    thread::{self, JoinHandle},
};

use bfc_ir::AstNode;

#[derive(Debug)]
pub enum RunTimeError {
    OutOfBoundsLeft,
    OutOfBoundsRight,
    MaxIterationsExceeded,
}

#[derive(Debug)]
pub struct Interpreter {
    instructions: Arc<Vec<AstNode>>,
    max_iterations: u64,
}

impl Interpreter {
    pub fn new(instructions: Vec<AstNode>, max_iterations: u64) -> Self {
        Self {
            instructions: Arc::new(instructions),
            max_iterations,
        }
    }

    /// Spawn a new machine and provide channels to communicate with it asynchronously
    pub fn spawn(&self) -> (InputTx, OutputRx, JoinHandle<()>) {
        let (input_tx, output_rx, inner) = self.create();

        let handle = inner.run();

        (input_tx, output_rx, handle)
    }

    /// Spawn a new interpreter and run it to completion with provide input
    pub fn run<I>(&self, inputs: I) -> Result<Vec<u8>, (Vec<u8>, RunTimeError)>
    where
        I: IntoIterator<Item = u8>,
    {
        let (input_tx, output_rx, inner) = self.create();

        inputs
            .into_iter()
            .map(|i| Wrapping(i))
            .for_each(|i| input_tx.send(i).unwrap());

        inner.run_blocking();

        let mut outputs = vec![];
        for output in output_rx.iter() {
            match output {
                Ok(b) => outputs.push(b.0),
                Err(err) => return Err((outputs, err)),
            }
        }

        Ok(outputs)
    }

    fn create(&self) -> (InputTx, OutputRx, InterpreterInner) {
        // Create two channels to handle inputs and outputs
        let (input_tx, input_rx): (InputTx, InputRx) = channel();
        let (output_tx, output_rx): (OutputTx, OutputRx) = channel();

        (
            input_tx,
            output_rx,
            InterpreterInner {
                instructions: self.instructions.clone(),
                max_iterations: self.max_iterations,
                memory: vec![Wrapping(0); 30000],
                memory_pointer: 0,
                iterations: 0,
                inputs: input_rx,
                outputs: output_tx,
            },
        )
    }
}

pub type InputTx = Sender<Wrapping<u8>>;
pub type InputRx = Receiver<Wrapping<u8>>;
pub type OutputTx = Sender<Result<Wrapping<u8>, RunTimeError>>;
pub type OutputRx = Receiver<Result<Wrapping<u8>, RunTimeError>>;

/// Interpreter that's receives inputs and sends outputs down channels
struct InterpreterInner {
    instructions: Arc<Vec<AstNode>>,
    max_iterations: u64,
    memory: Vec<Wrapping<u8>>,
    memory_pointer: isize,
    iterations: u64,

    inputs: InputRx,
    outputs: OutputTx,
}

impl InterpreterInner {
    fn run(mut self) -> thread::JoinHandle<()> {
        thread::spawn(move || {
            let _ = self.run_body(&self.instructions.clone());
        })
    }

    fn run_blocking(mut self) {
        let _ = self.run_body(&self.instructions.clone());
    }

    fn run_body(&mut self, body: &[AstNode]) -> Result<(), ()> {
        for instruction in body {
            self.iterations += 1;
            if self.iterations > self.max_iterations {
                self.outputs
                    .send(Err(RunTimeError::MaxIterationsExceeded))
                    .unwrap();
                return Err(());
            }

            match instruction {
                AstNode::Increment { amount, offset, .. } => {
                    let index = self.memory_pointer.checked_add(*offset).ok_or_else(|| {
                        self.outputs
                            .send(Err(RunTimeError::OutOfBoundsRight))
                            .unwrap()
                    })?;

                    // Convert isize to usize
                    let index = match index.cmp(&0) {
                        Ordering::Greater => index as usize,
                        Ordering::Equal => 0,
                        Ordering::Less => {
                            self.outputs
                                .send(Err(RunTimeError::OutOfBoundsLeft))
                                .unwrap();
                            return Err(());
                        }
                    };

                    // Check if the index is out of bounds
                    if index >= self.memory.len() {
                        self.outputs
                            .send(Err(RunTimeError::OutOfBoundsRight))
                            .unwrap();
                        return Err(());
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
                        self.outputs
                            .send(Err(RunTimeError::OutOfBoundsLeft))
                            .unwrap();
                        return Err(());
                    } else if self.memory_pointer.unsigned_abs() > self.memory.len() {
                        self.outputs
                            .send(Err(RunTimeError::OutOfBoundsRight))
                            .unwrap();
                        return Err(());
                    }
                }
                AstNode::Read { .. } => {
                    self.memory[self.memory_pointer as usize] = self.inputs.recv().unwrap();
                }
                AstNode::Write { .. } => {
                    self.outputs
                        .send(Ok(self.memory[self.memory_pointer as usize]))
                        .unwrap();
                }
                AstNode::Loop { body, .. } => {
                    while self.memory[self.memory_pointer as usize] != Wrapping(0) {
                        self.run_body(body)?;
                    }
                }
                AstNode::Set { amount, offset, .. } => {
                    let index = self.memory_pointer.checked_add(*offset).ok_or_else(|| {
                        self.outputs
                            .send(Err(RunTimeError::OutOfBoundsRight))
                            .unwrap()
                    })?;

                    // Convert isize to usize
                    let index = match index.cmp(&0) {
                        Ordering::Greater => index as usize,
                        Ordering::Equal => 0,
                        Ordering::Less => {
                            self.outputs
                                .send(Err(RunTimeError::OutOfBoundsLeft))
                                .unwrap();
                            return Err(());
                        }
                    };

                    // Check if the index is out of bounds
                    if index >= self.memory.len() {
                        self.outputs
                            .send(Err(RunTimeError::OutOfBoundsRight))
                            .unwrap();
                        return Err(());
                    }

                    // Convert the i8 to Wrapped u8
                    self.memory[index] = match amount.0.cmp(&0) {
                        Ordering::Less => -Wrapping(amount.0.unsigned_abs()),
                        Ordering::Equal => Wrapping(0),
                        Ordering::Greater => Wrapping(amount.0.unsigned_abs()),
                    };
                }
                AstNode::MultiplyMove { changes, .. } => {
                    let current = self.memory[self.memory_pointer as usize];

                    if current != Wrapping(0) {
                        for (offset, factor) in changes.iter() {
                            let index =
                                self.memory_pointer.checked_add(*offset).ok_or_else(|| {
                                    self.outputs
                                        .send(Err(RunTimeError::OutOfBoundsRight))
                                        .unwrap()
                                })?;

                            // Convert isize to usize
                            let index = match index.cmp(&0) {
                                Ordering::Greater => index as usize,
                                Ordering::Equal => 0,
                                Ordering::Less => {
                                    self.outputs
                                        .send(Err(RunTimeError::OutOfBoundsLeft))
                                        .unwrap();
                                    return Err(());
                                }
                            };

                            // Check if the index is out of bounds
                            if index >= self.memory.len() {
                                self.outputs
                                    .send(Err(RunTimeError::OutOfBoundsRight))
                                    .unwrap();
                                return Err(());
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
