//! # gtfsort
//! A fast and efficient GTF sorter tool.
//!
//! ## Overview
//! `gtfsort` is a rapid chr/pos/feature GTF2.5-3 sorter using a lexicographic-based
//! index ordering algorithm written in Rust. This tool is intended to be used as a
//! standalone command-line tool. The primary goal of this tool is to sort GTF files
//! by chromosome, position and feature in a fast and memory-efficient way.
//!
//! To use `gtfsort` as a standalone command-line tool, follow these steps:
//!
//! 1. install Rust from [here](https://www.rust-lang.org/tools/install)
//!
//! 2. install `gtfsort` by running:
//! ``` bash
//! cargo install gtfsort
//! ```
//!
//! 3. run `gtfsort` by typing:
//! ``` bash
//! gtfsort <input> <output> [<threads>]
//! ```

use clap::{self, Parser};
use colored::Colorize;
use log::Level;
use std::path::PathBuf;

use gtfsort::*;

#[derive(Parser, Debug)]
#[clap(
    name = "gtfsort",
    version = "0.2.2",
    author = "Alejandro Gonzales-Irribarren <jose.gonzalesdezavala1@unmsm.edu.pe>",
    about = "An optimized chr/pos/feature GTF2.5-3 sorter using a lexicographic-based index ordering algorithm written in Rust."
)]
struct Args {
    #[clap(
        short = 'i',
        long = "input",
        help = "Path to unsorted GTF file",
        value_name = "UNSORTED",
        required = true
    )]
    input: PathBuf,

    #[clap(
        short = 'o',
        long = "output",
        help = "Path to output sorted GTF file",
        value_name = "OUTPUT",
        required = true
    )]
    output: PathBuf,

    #[clap(
        short = 't',
        long,
        help = "Number of threads",
        value_name = "THREADS",
        default_value_t = num_cpus::get()
    )]
    threads: usize,
}

impl Args {
    /// Checks all the arguments for validity using validate_args()
    pub fn check(&self) -> Result<(), GtfSortError> {
        self.validate_args()
    }

    /// Checks the input file for validity. The file must exist and be a GTF or GFF3 file.
    /// If the file does not exist, an GtfSortError is returned.
    fn check_input(&self) -> Result<(), GtfSortError> {
        if !self.input.exists() {
            let err = format!("file {:?} does not exist", self.input);
            Err(GtfSortError::InvalidInput(err))
        } else if !self.input.extension().unwrap().eq("gff")
            & !self.input.extension().unwrap().eq("gtf")
            & !self.input.extension().unwrap().eq("gff3")
        {
            let err = format!(
                "file {:?} is not a GTF or GFF3 file, please specify the correct format",
                self.input
            );
            return Err(GtfSortError::InvalidInput(err));
        } else if std::fs::metadata(&self.input).unwrap().len() == 0 {
            let err = format!("file {:?} is empty", self.input);
            return Err(GtfSortError::InvalidInput(err));
        } else {
            Ok(())
        }
    }

    /// Checks the output file for validity. If the file is not a BED file, an GtfSortError is returned.
    fn check_output(&self) -> Result<(), GtfSortError> {
        if !self.output.extension().unwrap().eq("gtf")
            & !self.output.extension().unwrap().eq("gff3")
            & !self.output.extension().unwrap().eq("gff")
        {
            let err = format!(
                "file {:?} is not a GTF/GFF file, please specify the correct output format",
                self.output
            );
            Err(GtfSortError::InvalidOutput(err))
        } else {
            Ok(())
        }
    }

    /// Checks the number of threads for validity. The number of threads must be greater than 0
    /// and less than or equal to the number of logical CPUs.
    fn check_threads(&self) -> Result<(), GtfSortError> {
        if self.threads == 0 {
            let err = "number of threads must be greater than 0".to_string();
            Err(GtfSortError::InvalidThreads(err))
        } else if self.threads > num_cpus::get() {
            let err = "number of threads must be less than or equal to the number of logical CPUs"
                .to_string();
            Err(GtfSortError::InvalidThreads(err))
        } else {
            Ok(())
        }
    }

    /// Validates all the arguments
    fn validate_args(&self) -> Result<(), GtfSortError> {
        self.check_input()?;
        self.check_output()?;
        self.check_threads()?;
        Ok(())
    }
}

fn main() {
    simple_logger::init_with_level(Level::Info).unwrap();
    let args = Args::parse();
    args.check().unwrap_or_else(|e| {
        log::error!("{:?}", e);
        std::process::exit(1);
    });

    run(args);

    log::info!(
        "{} {}",
        "Success:".bright_green().bold(),
        "GTF file sorted successfully!"
    );
}

fn run(args: Args) {
    msg();

    let start = std::time::Instant::now();

    let job_info = sort_annotations(&args.input, &args.output, args.threads).unwrap_or_else(|e| {
        log::error!("{}: {}", "Fatal GtfSortError".bright_red().bold(), e);
        std::process::exit(1);
    });

    let elapsed = start.elapsed().as_secs_f32();
    log::info!("Elapsed time: {:.4} seconds", elapsed);
    log::info!(
        "Memory usage: {:.4} MB",
        job_info.end_mem_mb.unwrap_or(f64::NAN) - job_info.start_mem_mb.unwrap_or(f64::NAN)
    );
}
