use std::{
    fs,
    io::{self, BufRead, Write},
    num::Wrapping,
    process::exit,
    thread,
};

use bfi::{Interpreter, OptimisationsFlags};
use clap::Parser;

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(value_parser)]
    brainfuck: Option<String>,

    #[clap(short, long, value_parser, default_value = "true")]
    optimize: bool,

    #[clap(short, long, value_parser, default_value = "false")]
    raw: bool,

    #[clap(long, value_parser, default_value = "18446744073709551615")]
    max_iterations: u64,
}

fn main() {
    let args = Args::parse();

    // Attempt to parse the file as a path
    let program = match &args.brainfuck {
        Some(input) => match fs::read_to_string(input) {
            Ok(program) => program,
            Err(_) => input.to_string(),
        },
        None => {
            let mut buf = String::new();
            io::stdin().read_line(&mut buf).unwrap();
            buf
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
        }
    }

    let interpreter = Interpreter::new(instructions, args.max_iterations);
    let (tx, rx, handle) = interpreter.spawn();

    if args.raw {
        // On one thread read from stdin
        thread::spawn(move || {
            // lock stdin
            let mut stdin = io::stdin().lock();

            loop {
                let mut buffer = String::new();
                stdin.read_line(&mut buffer).unwrap();
                buffer
                    .split_whitespace()
                    .map(|s| s.parse())
                    .filter(|b| b.is_ok())
                    .for_each(|b| tx.send(Wrapping(b.unwrap())).unwrap())
            }
        });

        // On the another write to stdout
        thread::spawn(move || {
            let mut stdout = io::stdout().lock();
            while let Ok(b) = rx.recv() {
                match b {
                    Ok(b) => {
                        write!(stdout, "{} ", b).unwrap();
                    }
                    Err(err) => {
                        eprintln!("Runtime Error {:?}", err);
                        exit(1);
                    }
                }
            }
            stdout.flush().unwrap();
        });
    } else {
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
                match b {
                    Ok(b) => {
                        buf[0] = b.0;
                        stdout.write_all(&buf).unwrap();
                    }
                    Err(err) => {
                        eprintln!("Runtime Error {:?}", err);
                        exit(1);
                    }
                }
            }
            stdout.flush().unwrap();
        });
    }

    // Join the the VM
    handle.join().unwrap();
}
