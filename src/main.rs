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
#[cfg(feature = "mmap")]
use mmap::Madvice;
use rayon::prelude::*;
#[cfg(all(feature = "mmap", unix))]
use std::os::unix::fs::MetadataExt;
use std::{fs::File, path::PathBuf};
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

fn timed<T, F: FnOnce() -> T>(key: &str, f: F) -> T {
    let start = std::time::Instant::now();
    let res = f();
    let elapsed = start.elapsed().as_secs_f64();
    log::info!("{}: {:.2}s", key, elapsed);
    res
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
            Err(ArgError::InvalidInput(err))
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
            Err(ArgError::InvalidOutput(err))
        } else {
            Ok(())
        }
    }

    /// Checks the number of threads for validity. The number of threads must be greater than 0
    /// and less than or equal to the number of logical CPUs.
    fn check_threads(&self) -> Result<(), ArgError> {
        if self.threads == 0 {
            let err = "number of threads must be greater than 0".to_string();
            Err(ArgError::InvalidThreads(err))
        } else if self.threads > num_cpus::get() {
            let err = "number of threads must be less than or equal to the number of logical CPUs"
                .to_string();
            Err(ArgError::InvalidThreads(err))
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

    let input_ext = args
        .input
        .extension()
        .expect("Missing input file extension")
        .to_str()
        .expect("Invalid input file extension");

    let start = std::time::Instant::now();
    let start_mem = max_mem_usage_mb();

    rayon::ThreadPoolBuilder::new()
        .num_threads(args.threads)
        .build_global()
        .unwrap();

    log::info!("Using {} threads", args.threads);

    #[cfg(feature = "mmap")]
    let f = File::open(&args.input).unwrap_or_else(|e| {
        log::error!("{} {}", "Error:".bright_red().bold(), e);
        std::process::exit(1);
    });

    #[cfg(all(feature = "mmap", unix))]
    let contents_map = unsafe {
        mmap::MemoryMap::<u8>::from_file(
            &f,
            f.metadata().expect("Failed to get input file size.").size() as usize,
        )
        .unwrap_or_else(|e| {
            log::error!("{} {}", "Error:".bright_red().bold(), e);
            std::process::exit(1);
        })
    };

    #[cfg(all(feature = "mmap", windows))]
    let contents_map = unsafe {
        mmap::MemoryMap::<u8>::from_handle(&f, None).unwrap_or_else(|e| {
            log::error!("{} {}", "Error:".bright_red().bold(), e);
            std::process::exit(1);
        })
    };

    #[cfg(feature = "mmap")]
    match contents_map.madvise(&[Madvice::WillNeed, Madvice::Sequential, Madvice::HugePage]) {
        Ok(_) => {}
        Err(e) => {
            log::warn!("{} madvise: {}", "Warning:".bright_yellow().bold(), e);
        }
    }

    #[cfg(feature = "mmap")]
    let contents_ref = unsafe { std::str::from_utf8_unchecked(contents_map.as_slice()) };

    #[cfg(feature = "mmap")]
    log::info!(
        "Successfully mapped file to memory, size: {} bytes",
        contents_ref.len()
    );

    #[cfg(not(feature = "mmap"))]
    let contents = std::fs::read_to_string(&args.input).unwrap_or_else(|e| {
        log::error!("{} {}", "Error:".bright_red().bold(), e);
        std::process::exit(1);
    });
    #[cfg(not(feature = "mmap"))]
    let contents_ref = contents.as_str();

    let records = timed("Parsing input", || {
        match input_ext {
            "gff" | "gff3" => parallel_parse::<b'='>(contents_ref),
            "gtf" => parallel_parse::<b' '>(contents_ref),
            _ => Err("Unknown file extension, please specify a GTF or GFF3 file"),
        }
        .unwrap_or_else(|e| {
            log::error!("{} {}", "Error:".bright_red().bold(), e);
            std::process::exit(1);
        })
    });

    let index = DashMap::<&str, Layers>::new();

    timed("building index", || {
        records.par_iter().for_each(|(chrom, lines)| {
            let mut acc = Layers::default();

            for line in lines {
                match line.feat {
                    "gene" => {
                        acc.layer.push(line.outer_layer());
                    }
                    "transcript" => {
                        acc.mapper
                            .entry(line.gene_id)
                            .or_default()
                            .push(line.transcript_id);
                        acc.helper.entry(line.transcript_id).or_insert(line.line);
                    }
                    "CDS" | "exon" | "start_codon" | "stop_codon" => {
                        let (exon_number, suffix) = line.inner_layer();
                        acc.inner.entry(line.transcript_id).or_default().insert(
                            CowNaturalSort::new(format!("{}{}", exon_number, suffix).into()),
                            vec![line.line],
                        );
                    }
                    _ => {
                        acc.inner
                            .entry(line.transcript_id)
                            .or_default()
                            .entry(CowNaturalSort::new(line.feat.into()))
                            .and_modify(|e| {
                                e.push(line.line);
                            })
                            .or_insert(vec![line.line]);
                    }
                }
            }

            acc.layer.par_sort_unstable_by_key(|x| x.0);
            index.insert(chrom, acc);
        })
    });

    let mut keys: Vec<&str> = index.iter().map(|x| *x.key()).collect();
    keys.sort_by(|a, b| natord::compare(a, b));

    match timed("Writing output", || {
        write_obj(
            &args.output,
            &index,
            keys.iter()
                .map(|chr| (*chr, index.get(chr).unwrap().count_line_size()))
                .collect::<Vec<_>>(),
        )
    }) {
        Ok(_) => {}
        #[cfg(feature = "mmap")]
        Err(e) => {
            log::warn!(
                "{} {}",
                "Memory Mapped Write Output Error, falling back to sequential write:"
                    .bright_red()
                    .bold(),
                e
            );
            write_obj_sequential(
                &args.output,
                &index,
                keys.into_iter().map(|chr| (chr, 0)).collect(),
            )
            .unwrap_or_else(|e| {
                log::error!("{} {}", "Write Output Error:".bright_red().bold(), e);
                std::process::exit(1);
            });
        }
        #[cfg(not(feature = "mmap"))]
        Err(e) => {
            log::error!("{} {}", "Write Output Error:".bright_red().bold(), e);
            std::process::exit(1);
        }
    }

    let elapsed = start.elapsed().as_secs_f32();
    let mem = (max_mem_usage_mb() - start_mem).max(0.0);
    log::info!("Elapsed time: {:.4} seconds", elapsed);
    log::info!("Memory usage: {:.4} MB", mem);
}
