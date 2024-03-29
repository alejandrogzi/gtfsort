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
use dashmap::DashMap;
use log::Level;
use natord::compare;
use num_cpus;
use rayon::prelude::*;
use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;

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
    pub fn check(&self) -> Result<(), ArgError> {
        self.validate_args()
    }

    /// Checks the input file for validity. The file must exist and be a GTF or GFF3 file.
    /// If the file does not exist, an error is returned.
    fn check_input(&self) -> Result<(), ArgError> {
        if !self.input.exists() {
            let err = format!("file {:?} does not exist", self.input);
            return Err(ArgError::InvalidInput(err));
        } else if !self.input.extension().unwrap().eq("gff")
            & !self.input.extension().unwrap().eq("gtf")
            & !self.input.extension().unwrap().eq("gff3")
        {
            let err = format!(
                "file {:?} is not a GTF or GFF3 file, please specify the correct format",
                self.input
            );
            return Err(ArgError::InvalidInput(err));
        } else if std::fs::metadata(&self.input).unwrap().len() == 0 {
            let err = format!("file {:?} is empty", self.input);
            return Err(ArgError::InvalidInput(err));
        } else {
            Ok(())
        }
    }

    /// Checks the output file for validity. If the file is not a BED file, an error is returned.
    fn check_output(&self) -> Result<(), ArgError> {
        if !self.output.extension().unwrap().eq("gtf")
            & !self.output.extension().unwrap().eq("gff3")
            & !self.output.extension().unwrap().eq("gff")
        {
            let err = format!(
                "file {:?} is not a GTF/GFF file, please specify the correct output format",
                self.output
            );
            return Err(ArgError::InvalidOutput(err));
        } else {
            Ok(())
        }
    }

    /// Checks the number of threads for validity. The number of threads must be greater than 0
    /// and less than or equal to the number of logical CPUs.
    fn check_threads(&self) -> Result<(), ArgError> {
        if self.threads == 0 {
            let err = format!("number of threads must be greater than 0");
            return Err(ArgError::InvalidThreads(err));
        } else if self.threads > num_cpus::get() {
            let err = format!(
                "number of threads must be less than or equal to the number of logical CPUs"
            );
            return Err(ArgError::InvalidThreads(err));
        } else {
            Ok(())
        }
    }

    /// Validates all the arguments
    fn validate_args(&self) -> Result<(), ArgError> {
        self.check_input()?;
        self.check_output()?;
        self.check_threads()?;
        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum ArgError {
    /// The input file does not exist or is not a GTF or GFF3 file.
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// The output file is not a BED file.
    #[error("Invalid output: {0}")]
    InvalidOutput(String),

    /// The number of threads is invalid.
    #[error("Invalid number of threads: {0}")]
    InvalidThreads(String),
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
    let start_mem = max_mem_usage_mb();

    rayon::ThreadPoolBuilder::new()
        .num_threads(args.threads)
        .build_global()
        .unwrap();

    log::info!("Using {} threads", args.threads);

    let contents = reader(&args.input).unwrap_or_else(|e| {
        log::error!("{} {}", "Error:".bright_red().bold(), e);
        std::process::exit(1);
    });
    let records = parallel_parse(&contents).unwrap_or_else(|e| {
        log::error!("{} {}", "Error:".bright_red().bold(), e);
        std::process::exit(1);
    });

    let index = DashMap::<Arc<str>, Layers>::new();

    records.par_iter().for_each(|(chrom, lines)| {
        let mut acc = Layers::default();

        for line in lines {
            match line.feat.as_str() {
                "gene" => {
                    acc.layer.push(line.outer_layer());
                }
                "transcript" => {
                    acc.mapper
                        .entry(line.gene_id.clone())
                        .or_default()
                        .push(line.transcript_id.clone());
                    acc.helper
                        .entry(line.transcript_id.clone())
                        .or_insert(line.line.clone());
                }
                "CDS" | "exon" | "start_codon" | "stop_codon" => {
                    let exon_number = line.inner_layer();
                    acc.inner
                        .entry(line.transcript_id.clone())
                        .or_default()
                        .insert(
                            Sort::new(exon_number.as_str()),
                            line.line.clone().to_string(),
                        );
                }
                _ => {
                    acc.inner
                        .entry(line.transcript_id.clone())
                        .or_default()
                        .entry(Sort::new(line.feat.clone().as_str()))
                        .and_modify(|e| {
                            e.push('\n');
                            e.push_str(&line.line.clone());
                        })
                        .or_insert(line.line.clone().to_string());
                }
            }
        }

        acc.layer.par_sort_unstable_by_key(|x| x.0);
        index.insert(chrom.clone(), acc);
    });

    let mut keys: Vec<Arc<str>> = index.iter().map(|x| x.key().clone()).collect();
    keys.par_sort_unstable_by(|a, b| compare(a, b));

    let _ = write_obj(&args.output, &index, &keys);

    let elapsed = start.elapsed().as_secs_f32();
    let mem = (max_mem_usage_mb() - start_mem).max(0.0);
    log::info!("Elapsed time: {:.4} seconds", elapsed);
    log::info!("Memory usage: {:.4} MB", mem);
}
