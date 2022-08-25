mod interpreter;

#[cfg(test)]
mod tests;

pub use bfc_ir::{optimize, parse, OptimisationsFlags};
pub use interpreter::Interpreter;

pub enum Error {
    ParseError(bfc_ir::ParseError),
    RunTimeError(),
}
