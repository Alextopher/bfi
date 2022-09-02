use crate::{test_blocking, TestResult, TestResults};
use std::path::Path;

fn test_file<P: AsRef<Path>>(program: P, output: P) {
    // Read the file
    let program = std::fs::read_to_string(program).unwrap();
    let expected: Vec<u8> = std::fs::read_to_string(output).unwrap().bytes().collect();

    match test_blocking(&program, vec![], expected, u64::MAX) {
        TestResults::OutputsDontMatchInputs => unreachable!(),
        TestResults::ParseError(e) => panic!("failed to compile program {:?}", e),
        TestResults::Results(results) => {
            for r in results {
                match r {
                    TestResult::Ok => {}
                    TestResult::RunTimeError(e) => panic!("RunTimeError {:?}", e),
                    TestResult::UnexpectedOutput { expected, output } => {
                        assert_eq!(expected, output)
                    }
                }
            }
        }
    }
}

#[test]
fn inputs() {
    test_blocking(",.,.,.", vec![1, 2, 3], vec![1, 2, 3], u64::MAX);
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
    );
}

#[test]
fn mandelbrot_bf() {
    test_file(
        "sample_programs/mandelbrot.bf",
        "sample_programs/mandelbrot.bf.out",
    );
}

#[test]
fn multiply_bf() {
    test_file(
        "sample_programs/multiply.bf",
        "sample_programs/multiply.bf.out",
    );
}
