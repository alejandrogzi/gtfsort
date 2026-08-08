//! # gtfsort
//! A fast and efficient GTF sorter tool.
//!
//! ## Overview
//! `gtfsort` is a rapid chr/pos/feature GTF/GFF sorter using a lexicographic-based
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
//! gtfsort [--input <input>] [--output <output>] [--threads <threads>]
//! ```

use clap::{self, Parser};
use colored::Colorize;
use flate2::{read::GzDecoder, write::GzEncoder, Compression};
use log::Level;
use std::{
    fs::File,
    io::{self, Read, Write},
    path::{Path, PathBuf},
};

use gtfsort::*;

#[derive(Parser, Debug)]
#[clap(
    name = "gtfsort",
    version = env!("CARGO_PKG_VERSION"),
    author = "alejandrogzi <alejandrxgzi@gmail.com>, eternal-flame-AD <yume@yumechi.jp>",
    about = "An optimized chr/pos/feature GTF2.5-3 sorter using a lexicographic-based index ordering algorithm written in Rust."
)]
struct Args {
    #[clap(
        short = 'i',
        long = "input",
        help = "Path to unsorted GTF/GFF file; reads stdin when omitted or '-'",
        value_name = "UNSORTED"
    )]
    input: Option<PathBuf>,

    #[clap(
        short = 'o',
        long = "output",
        help = "Path to sorted GTF/GFF file; writes stdout when omitted or '-'",
        value_name = "OUTPUT"
    )]
    output: Option<PathBuf>,

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
        let Some(input) = stream_path(&self.input) else {
            return Ok(());
        };

        if !input.exists() {
            let err = format!("file {input:?} does not exist");
            Err(GtfSortError::InvalidInput(err))
        } else if !matches!(annotation_extension(input), Some("gff" | "gtf" | "gff3")) {
            let err = format!(
                "file {:?} is not a GTF or GFF3 file, please specify the correct format",
                input
            );
            Err(GtfSortError::InvalidInput(err))
        } else if std::fs::metadata(input)
            .map_err(|e| GtfSortError::IoError("reading input file metadata", e))?
            .len()
            == 0
        {
            let err = format!("file {input:?} is empty");
            Err(GtfSortError::InvalidInput(err))
        } else {
            Ok(())
        }
    }

    /// Checks the output file for validity. If the file is not a BED file, an GtfSortError is returned.
    fn check_output(&self) -> Result<(), GtfSortError> {
        let Some(output) = stream_path(&self.output) else {
            return Ok(());
        };

        if !matches!(annotation_extension(output), Some("gtf" | "gff3" | "gff")) {
            let err = format!(
                "file {:?} is not a GTF/GFF file, please specify the correct output format",
                output
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

/// Parses CLI arguments, runs the sorter, and reports a terminal status.
fn main() {
    simple_logger::init_with_level(Level::Info).unwrap();
    let args = Args::parse();
    args.check().unwrap_or_else(|e| {
        log::error!("{:?}", e);
        std::process::exit(1);
    });

    run(args).unwrap_or_else(|e| {
        log::error!("{}: {}", "Fatal GtfSortError".bright_red().bold(), e);
        std::process::exit(1);
    });

    log::info!(
        "{} {}",
        "Success:".bright_green().bold(),
        "GTF file sorted successfully!"
    );
}

/// Executes one validated sorting job and logs timing and memory statistics.
fn run(args: Args) -> Result<(), GtfSortError> {
    msg();

    let start = std::time::Instant::now();
    let input = stream_path(&args.input);
    let output = stream_path(&args.output);

    let (start_mem_mb, end_mem_mb) = match (input, output) {
        (Some(input), Some(output)) => {
            let job = sort_annotations(input, output, args.threads)?;
            (job.start_mem_mb, job.end_mem_mb)
        }
        _ => run_streamed(input, output, args.threads)?,
    };

    let elapsed = start.elapsed().as_secs_f32();
    log::info!("Elapsed time: {:.4} seconds", elapsed);
    log::info!(
        "Memory usage: {:.4} MB",
        end_mem_mb.unwrap_or(f64::NAN) - start_mem_mb.unwrap_or(f64::NAN)
    );

    Ok(())
}

#[derive(Clone, Copy)]
enum StreamFormat {
    Gtf,
    Gff,
}

/// Treats an omitted path or `-` as the corresponding standard stream.
fn stream_path(path: &Option<PathBuf>) -> Option<&Path> {
    path.as_deref().filter(|path| *path != Path::new("-"))
}

/// Runs a job where at least one endpoint is a standard stream.
fn run_streamed(
    input: Option<&Path>,
    output: Option<&Path>,
    threads: usize,
) -> Result<(Option<f64>, Option<f64>), GtfSortError> {
    let contents = read_input(input)?;
    if contents.is_empty() {
        return Err(GtfSortError::InvalidInput("stdin is empty".to_string()));
    }

    let format = stream_format(input, output, &contents);

    if let Some(path) = output {
        let file =
            File::create(path).map_err(|e| GtfSortError::IoError("creating output file", e))?;

        if is_gzip_path(path) {
            let mut writer = GzEncoder::new(file, Compression::default());
            let memory = sort_to_writer(&contents, &mut writer, format, threads)?;
            writer
                .finish()
                .map_err(|e| GtfSortError::IoError("finishing compressed output", e))?;
            Ok(memory)
        } else {
            let mut writer = file;
            let memory = sort_to_writer(&contents, &mut writer, format, threads)?;
            writer
                .flush()
                .map_err(|e| GtfSortError::IoError("flushing output file", e))?;
            Ok(memory)
        }
    } else {
        let stdout = io::stdout();
        let mut writer = stdout.lock();
        let memory = sort_to_writer(&contents, &mut writer, format, threads)?;
        writer
            .flush()
            .map_err(|e| GtfSortError::IoError("flushing stdout", e))?;
        Ok(memory)
    }
}

/// Reads plain stdin or a named input, retaining suffix-driven gzip support.
fn read_input(path: Option<&Path>) -> Result<String, GtfSortError> {
    let mut contents = String::new();

    match path {
        Some(path) if is_gzip_path(path) => {
            let file =
                File::open(path).map_err(|e| GtfSortError::IoError("opening input file", e))?;
            GzDecoder::new(file)
                .read_to_string(&mut contents)
                .map_err(|e| GtfSortError::IoError("decompressing input file", e))?;
        }
        Some(path) => {
            File::open(path)
                .map_err(|e| GtfSortError::IoError("opening input file", e))?
                .read_to_string(&mut contents)
                .map_err(|e| GtfSortError::IoError("reading input file", e))?;
        }
        None => {
            io::stdin()
                .read_to_string(&mut contents)
                .map_err(|e| GtfSortError::IoError("reading stdin", e))?;
        }
    }

    Ok(contents)
}

/// Selects the attribute parser without adding another CLI option.
fn stream_format(input: Option<&Path>, output: Option<&Path>, contents: &str) -> StreamFormat {
    if let Some(format) = input.and_then(format_from_path) {
        return format;
    }

    if contents.lines().any(|line| {
        line.trim_start().starts_with("##gff-version")
            || line.split('\t').nth(8).is_some_and(|attributes| {
                attributes
                    .split(';')
                    .any(|field| field.trim_start().starts_with("gene_id="))
            })
    }) {
        return StreamFormat::Gff;
    }

    output
        .and_then(format_from_path)
        .unwrap_or(StreamFormat::Gtf)
}

fn format_from_path(path: &Path) -> Option<StreamFormat> {
    match annotation_extension(path) {
        Some("gff" | "gff3") => Some(StreamFormat::Gff),
        Some("gtf") => Some(StreamFormat::Gtf),
        _ => None,
    }
}

/// Adapts the existing callback API to an ordinary writer.
fn sort_to_writer<W: Write>(
    contents: &str,
    writer: &mut W,
    format: StreamFormat,
    threads: usize,
) -> Result<(Option<f64>, Option<f64>), GtfSortError> {
    let mut output = |bytes: &[u8]| writer.write(bytes);
    let job = match format {
        StreamFormat::Gtf => sort_annotations_string::<b' ', _>(contents, &mut output, threads)?,
        StreamFormat::Gff => sort_annotations_string::<b'=', _>(contents, &mut output, threads)?,
    };

    Ok((job.start_mem_mb, job.end_mem_mb))
}
