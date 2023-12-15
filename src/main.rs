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

use std::collections::BTreeMap;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

use colored::Colorize;

use num_cpus;

use log::Level;

use natord::compare;

use clap::{self, Parser};

use rayon::prelude::*;

use gtfsort::*;

#[derive(Parser, Debug)]
#[clap(
    name = "gtfsort",
    version = "0.2.1",
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

fn main() {
    let args = Args::parse();

    if args.threads == 0 {
        println!(
            "{} {}",
            "Error:".bright_red().bold(),
            "Number of threads must be greater than 0!"
        );
        std::process::exit(1);
    }

    if std::fs::metadata(&args.input).unwrap().len() == 0 {
        eprintln!("Error: input file is empty");
        std::process::exit(1);
    }

    if args.input == args.output {
        eprintln!("Error: input and output files must be different");
        std::process::exit(1);
    }

    run(args);

    println!(
        "{} {}",
        "Success:".bright_green().bold(),
        "GTF file sorted successfully!"
    );
}

fn run(args: Args) {
    msg();
    let start = std::time::Instant::now();
    let start_mem = max_mem_usage_mb();
    simple_logger::init_with_level(Level::Info).unwrap();

    rayon::ThreadPoolBuilder::new()
        .num_threads(args.threads)
        .build_global()
        .unwrap();

    log::info!("Using {} threads", args.threads);

    let contents = reader(&args.input).unwrap_or_else(|e| {
        eprintln!("{} {}", "Error:".bright_red().bold(), e);
        std::process::exit(1);
    });
    let records = parallel_parse(&contents).unwrap_or_else(|e| {
        eprintln!("{} {}", "Error:".bright_red().bold(), e);
        std::process::exit(1);
    });

    let file = File::create(&args.output).unwrap();
    let mut output = BufWriter::new(file);

    let mut layer: Vec<(String, i32, String, String)> = vec![];
    let mut mapper: HashMap<String, Vec<String>> = HashMap::new();
    let mut inner: HashMap<String, BTreeMap<Sort, String>> = HashMap::new();
    let mut helper: HashMap<String, String> = HashMap::new();

    log::info!("Sorting GTF file...");
    for record in records {
        if record.chrom.is_empty() {
            writeln!(output, "{}", record.line).unwrap();
            continue;
        }

        match record.feature() {
            "gene" => {
                layer.push(record.outer_layer());
            }
            "transcript" => {
                let (gene, transcript, line) = record.gene_to_transcript();
                mapper
                    .entry(gene)
                    .or_insert(Vec::new())
                    .push(transcript.clone());
                helper.entry(transcript).or_insert(line);
            }
            "CDS" | "exon" | "start_codon" | "stop_codon" => {
                let (transcript, exon_number, line) = record.inner_layer();
                inner
                    .entry(transcript)
                    .or_insert(BTreeMap::new())
                    .insert(Sort::new(exon_number.as_str()), line);
            }
            _ => {
                let (transcript, feature, line) = record.misc_layer();
                inner
                    .entry(transcript)
                    .or_insert_with(|| BTreeMap::new())
                    .entry(Sort::new(feature.as_str()))
                    .and_modify(|e| {
                        e.push('\n');
                        e.push_str(&line);
                    })
                    .or_insert(line);
            }
        };
    }

    layer.par_sort_unstable_by(|a, b| {
        let cmp_chr = compare(&a.0, &b.0);
        if cmp_chr == std::cmp::Ordering::Equal {
            a.1.cmp(&b.1)
        } else {
            cmp_chr
        }
    });

    for i in layer {
        writeln!(output, "{}", i.3).unwrap();

        let transcripts = mapper
            .get(&i.2)
            .ok_or("Error: genes with 0 transcripts are not allowed")
            .unwrap();
        for j in transcripts.iter() {
            writeln!(output, "{}", helper.get(j).unwrap()).unwrap();
            let exons = inner
                .get(j)
                .ok_or("Error: transcripts with 0 exons are not allowed")
                .unwrap();
            let joined_exons: String = exons
                .values()
                .map(|value| value.to_string())
                .collect::<Vec<String>>()
                .join("\n");
            writeln!(output, "{}", joined_exons).unwrap();
        }
    }

    output.flush().unwrap();

    let elapsed = start.elapsed().as_secs_f32();
    let mem = (max_mem_usage_mb() - start_mem).max(0.0);
    log::info!("Elapsed time: {:.4} seconds", elapsed);
    log::info!("Memory usage: {:.4} MB", mem);
}
