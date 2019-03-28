use std::env;
use std::fs;
use std::io;
use std::io::Write;

use clap::{App, Arg};
use env_logger::{fmt, Builder, Target};
use failure::Error;
use log::{error, info};
use log::{Level, LevelFilter, Record};

use varisat::solver::{ProofFormat, Solver};

fn main() {
    let exit_code = match main_with_err() {
        Err(err) => {
            error!("{}", err);
            1
        }
        Ok(exit_code) => exit_code,
    };
    std::process::exit(exit_code);
}

pub fn main_with_err() -> Result<i32, Error> {
    let matches = App::new("varisat")
        .version(env!("VARISAT_VERSION"))
        .arg_from_usage("[INPUT] 'The input file to use (stdin if omitted)'")
        .arg_from_usage("[proof-file] --proof=[FILE] 'Write a proof to the specified file'")
        .arg(
            Arg::from_usage(
                "[proof-format] --proof-format=[FORMAT] 'Specify the proof format to use.'",
            )
            .possible_values(&["drat", "binary-drat", "varisat"])
            .default_value("drat")
            .case_insensitive(true),
        )
        .get_matches();

    let format = |buf: &mut fmt::Formatter, record: &Record| {
        if record.level() == Level::Info {
            writeln!(buf, "c {}", record.args())
        } else {
            writeln!(buf, "c {}: {}", record.level(), record.args())
        }
    };

    let mut builder = Builder::new();
    builder
        .target(Target::Stdout)
        .format(format)
        .filter(None, LevelFilter::Info);

    if let Ok(ref env_var) = env::var("VARISAT_LOG") {
        builder.parse_filters(env_var);
    }

    builder.init();

    info!("This is varisat {}", env!("VARISAT_VERSION"));
    info!(
        "  {} build - {}",
        env!("VARISAT_PROFILE"),
        env!("VARISAT_RUSTC_VERSION")
    );

    let mut solver = Solver::new();

    let stdin = io::stdin();

    let mut locked_stdin;
    let mut opened_file;

    let file = match matches.value_of("INPUT") {
        Some(path) => {
            info!("Reading file '{}'", path);
            opened_file = fs::File::open(path)?;
            &mut opened_file as &mut io::Read
        }
        None => {
            info!("Reading from stdin");
            locked_stdin = stdin.lock();
            &mut locked_stdin as &mut io::Read
        }
    };

    if let Some(path) = matches.value_of("proof-file") {
        let proof_format_str = matches
            .value_of("proof-format")
            .unwrap()
            .to_ascii_lowercase();

        let proof_format = match &proof_format_str[..] {
            "drat" => ProofFormat::Drat,
            "binary-drat" => ProofFormat::BinaryDrat,
            "varisat" => ProofFormat::Varisat,
            _ => unreachable!(),
        };

        info!("Writing {} proof to file '{}'", proof_format_str, path);

        solver.write_proof(fs::File::create(path)?, proof_format);
    }

    solver.add_dimacs_cnf(file)?;

    match solver.solve() {
        Some(true) => {
            println!("s SATISFIABLE");
            print!("v");
            for l in solver.model().unwrap() {
                print!(" {}", l);
            }
            println!(" 0");
            Ok(10)
        }
        Some(false) => {
            println!("s UNSATISFIABLE");
            Ok(20)
        }
        None => {
            println!("s UNKNOWN");
            Ok(0)
        }
    }
}
