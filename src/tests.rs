use std::{path::Path, thread};

use bfc_ir::OptimisationsFlags;

use crate::Interpreter;

fn test_file<P: AsRef<Path>>(program: P, output: P) {
    // Read the file
    let program = std::fs::read_to_string(program).unwrap();
    let expected = std::fs::read_to_string(output).unwrap();

    // Compile the program
    let instructions = bfc_ir::parse(&program).unwrap();

    // Optimize the program
    let flags = OptimisationsFlags::all();

    let (optimized, errors) = bfc_ir::optimize(instructions, flags);
    assert!(errors.is_empty());

    // Prepare the interpreter
    let (_tx, rx, interpreter) = Interpreter::new(&optimized, 10000000000000);

    let output = thread::spawn(move || {
        // receive from rx into a read_to_string
        let mut s = String::new();
        for b in rx.iter().map(|b| b.0 as char) {
            s.push(b);
        }
        s
    });

    interpreter.run().unwrap();

    assert_eq!(expected, output.join().unwrap());
}

#[test]
fn bang_bang_bf() {
    test_file(
        "sample_programs/bangbang.bf",
        "sample_programs/bangbang.bf.out",
    );
}

#[test]
fn bottles_bf() {
    test_file(
        "sample_programs/bottles.bf",
        "sample_programs/bottles.bf.out",
    );
}

#[test]
fn hello_world_bf() {
    test_file(
        "sample_programs/hello_world.bf",
        "sample_programs/hello_world.bf.out",
    )
}

#[test]
fn mandelbrot_bf() {
    test_file(
        "sample_programs/mandelbrot.bf",
        "sample_programs/mandelbrot.bf.out",
    )
}

#[test]
fn multiply_bf() {
    test_file(
        "sample_programs/multiply.bf",
        "sample_programs/multiply.bf.out",
    )
}
