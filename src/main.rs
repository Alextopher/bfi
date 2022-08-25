use std::{
    fs,
    io::{self, BufRead, Write},
    num::Wrapping,
    path::PathBuf,
    process::exit,
    thread,
};

use bfi::{Interpreter, OptimisationsFlags};
use clap::Parser;

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(value_parser)]
    file: PathBuf,

    #[clap(short, long, value_parser, default_value = "true")]
    optimize: bool,

    #[clap(long, value_parser, default_value = "18446744073709551615")]
    max_iterations: u64,
}

fn main() {
    let args = Args::parse();

    let program = match fs::read_to_string(&args.file) {
        Ok(program) => program,
        Err(err) => {
            eprintln!("{}", err);
            exit(1);
        }
    };

    let mut instructions = match bfc_ir::parse(&program) {
        Ok(instructions) => instructions,
        Err(err) => {
            eprintln!("{:?}", err);
            exit(1)
        }
    };

    if args.optimize {
        let flags = OptimisationsFlags::all();
        let warnings;
        (instructions, warnings) = bfc_ir::optimize(instructions, flags);

        if !warnings.is_empty() {
            for err in warnings {
                eprintln!("{:?}", err);
            }
            exit(1);
        }
    }

    let (tx, rx, interpreter) = Interpreter::new(&instructions, args.max_iterations);

    // On one thread read from stdin
    thread::spawn(move || {
        // lock stdin
        let mut stdin = io::stdin().lock();

        loop {
            let mut buffer = String::new();
            stdin.read_line(&mut buffer).unwrap();
            buffer.bytes().for_each(|b| tx.send(Wrapping(b)).unwrap())
        }
    });

    // On the another write to stdout
    thread::spawn(move || {
        let mut stdout = io::stdout().lock();
        let mut buf: [u8; 1] = [0];
        while let Ok(b) = rx.recv() {
            buf[0] = b.0;
            stdout.write_all(&buf).unwrap();
        }
        stdout.flush().unwrap();
    });

    // Run the program
    interpreter.run().unwrap();
}
