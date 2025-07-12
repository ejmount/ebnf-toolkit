#![warn(clippy::pedantic)]
#![warn(unused_crate_dependencies)]
/// This file has no public functionality - it runs the code coverage analysis
use std::env::set_current_dir;
use std::path::{Path, PathBuf};

use anyhow::Error;
use clap::{Parser, Subcommand};
use colored::{Color, Colorize};
use duct::cmd;
use fs_extra::dir::{create_all, get_dir_content};
use fs_extra::file::remove;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    subcommand: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run tests and compile coverage reports
    Coverage {
        /// Output coverage results as HTML rather than .lcov
        #[arg(short, long)]
        readable: bool,
        /// Whether to open HTML reports - only used if `readable` is true.
        #[arg(short, long)]
        open_report: bool,
    },
}

fn main() -> Result<(), Error> {
    let cli = Cli::parse();

    match cli.subcommand {
        Commands::Coverage {
            readable,
            open_report,
        } => coverage(readable, open_report),
    }
}

fn coverage(readable: bool, open_report: bool) -> Result<(), Error> {
    set_current_dir(root_crate_dir())?;

    create_all("coverage", true)?;

    set_current_dir(root_crate_dir())?;
    print!("Running tests... ");
    cmd!("cargo", "test")
        .env("CARGO_INCREMENTAL", "0")
        .env("RUSTFLAGS", "-Cinstrument-coverage")
        .env("LLVM_PROFILE_FILE", "coverage/cargo-test-%p-%m.profraw")
        .run()?;
    println!("{}", "ok".color(Color::Green));

    let (fmt, file) = if readable {
        ("html", "coverage/html")
    } else {
        ("lcov", "coverage/tests.lcov")
    };

    //set_current_dir(root_crate_dir())?;

    let (option_text, hash) =
        if let Ok(result) = cmd!("git", "rev-parse", "HEAD").stdout_capture().run() {
            ("--commit-sha", String::from_utf8(result.stdout).unwrap())
            //("", String::new())
        } else {
            ("", String::new())
        };
    dbg!(option_text, &hash);

    print!("Generating reports as {fmt}... ");
    cmd!(
        "grcov",
        ".",
        "--binary-path",
        "./target/debug/deps",
        "-s",
        ".",
        option_text,
        hash,
        "-t",
        fmt,
        "-o",
        file,
        "--branch",
        "--llvm",
        "--ignore-not-existing",
        "--ignore",
        "**/tests/*",
        "--ignore",
        "xtask/*",
        "--excl-start",
        "mod tests?",
        "--excl-line",
        "derive|unreachable",
    )
    .run()?;
    println!("{}", "ok".color(Color::Green));

    if readable {
        let index_file = format!("{file}/index.html");

        if open_report {
            match open::that(&index_file) {
                Ok(()) => {
                    println!("{}", "Opened".color(Color::Green));
                }
                Err(e) => {
                    eprintln!("{e}\n{} to open reports", "Failure".color(Color::Red));
                }
            }
        } else {
            let abs_path = Path::new(&index_file).canonicalize()?;
            println!("report location: {}", abs_path.to_string_lossy());
        }
    }
    print!("Cleaning up... ");
    let dir_content = get_dir_content(".")?;
    for prof_file in dir_content.files.iter().filter(|s| s.ends_with("profraw")) {
        remove(prof_file)?;
    }
    println!("{}", "ok".color(Color::Green));
    Ok(())
}

/// Get the root folder of the larger crate, assuming this is part of a
/// workspace
fn root_crate_dir() -> PathBuf {
    let mut xtask_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    xtask_dir.pop();
    xtask_dir
}
