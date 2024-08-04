pub mod gtf;

pub use gtf::Record;

pub mod ord;
pub use ord::CowNaturalSort;

pub mod utils;
use thiserror::Error;
pub use utils::*;

pub mod interop;

#[cfg(feature = "testing")]
pub mod test_utils;

use std::{io, path::PathBuf};

#[cfg(feature = "mmap")]
use mmap::Madvice;
#[cfg(feature = "mmap")]
use std::{borrow::Cow, fs::File};

#[allow(unused_imports)]
use colored::Colorize;
use dashmap::DashMap;
use rayon::prelude::*;

#[cfg(feature = "mmap")]
pub mod mmap;

#[derive(Debug, Error)]
pub enum GtfSortError {
    /// The input file does not exist or is not a GTF or GFF3 file.
    #[error("Invalid input: {0}")]
    InvalidInput(String),

    /// The output file is not a BED file.
    #[error("Invalid output: {0}")]
    InvalidOutput(String),

    /// Cannot parse the input file.
    #[error("Parse GtfSortError: {0}")]
    ParseError(&'static str),

    /// The number of threads is invalid.
    #[error("Invalid number of threads: {0}")]
    InvalidThreads(String),

    /// An IO GtfSortError occurred.
    #[error("IO GtfSortError: while {0}: {1}")]
    IoError(&'static str, std::io::Error),

    /// An Invalid Parameter is passed.
    #[error("Invalid parameter: {0}")]
    InvalidParameter(&'static str),
}

pub struct SortAnnotationsJobResult<'a> {
    pub input: &'a str,
    pub output: &'a str,
    pub threads: usize,
    pub input_mmaped: bool,
    pub output_mmaped: bool,
    pub parsing_secs: f64,
    pub indexing_secs: f64,
    pub writing_secs: f64,
    pub start_mem_mb: Option<f64>,
    pub end_mem_mb: Option<f64>,
}

pub fn sort_annotations<'a>(
    input: &'a PathBuf,
    output: &'a PathBuf,
    threads: usize,
) -> Result<SortAnnotationsJobResult<'a>, GtfSortError> {
    assert!(threads > 0, "Invalid number of threads");
    let mut ret = SortAnnotationsJobResult {
        input: input.to_str().ok_or(GtfSortError::InvalidInput(
            "Invalid input file path".to_string(),
        ))?,
        output: output.to_str().ok_or(GtfSortError::InvalidOutput(
            "Invalid output file path".to_string(),
        ))?,
        threads,
        input_mmaped: false,
        output_mmaped: false,
        parsing_secs: f64::NAN,
        indexing_secs: f64::NAN,
        writing_secs: f64::NAN,
        start_mem_mb: None,
        end_mem_mb: None,
    };

    let input_ext = input
        .extension()
        .ok_or(GtfSortError::InvalidInput(
            "Missing input file extension".to_string(),
        ))?
        .to_str()
        .ok_or(GtfSortError::InvalidInput(
            "Invalid input file extension".to_string(),
        ))?;

    let tp = rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .expect("Failed to build thread pool");

    tp.install(|| {
        ret.start_mem_mb = Some(max_mem_usage_mb());

        log::info!("Using {} threads", threads);

        #[cfg(feature = "mmap")]
        let f = File::open(input).map_err(|e| GtfSortError::IoError("opening input file", e))?;

        #[cfg(feature = "mmap")]
        let f_size = f
            .metadata()
            .map_err(|e| GtfSortError::IoError("getting input file metadata", e))?
            .len();

        #[cfg(feature = "mmap")]
        let mmap_result = (|| {
            #[cfg(feature = "mmap")]
            #[cfg(unix)]
            let contents_map = unsafe {
                mmap::MemoryMap::<u8>::from_file(&f, f_size as usize)
                    .map_err(|e| GtfSortError::IoError("mapping input file to memory", e))?
            };

            #[cfg(windows)]
            let contents_map = unsafe {
                mmap::MemoryMap::<u8>::from_handle(&f, f_size as usize)
                    .map_err(|e| GtfSortError::IoError("mapping input file to memory", e))?
            };

            match contents_map.madvise(&[Madvice::WillNeed, Madvice::Sequential, Madvice::HugePage])
            {
                Ok(_) => {}
                Err(e) => {
                    log::warn!("{} madvise: {}", "Warning:".bright_yellow().bold(), e);
                }
            }

            ret.input_mmaped = true;
            log::info!(
                "Successfully mapped file to memory, size: {} bytes",
                contents_map.size_bytes()
            );

            Ok::<_, GtfSortError>(contents_map)
        })();

        #[cfg(feature = "mmap")]
        let contents = match mmap_result.as_ref() {
            Ok(m) => Cow::Borrowed(unsafe { std::str::from_utf8_unchecked(m.as_slice()) }),
            Err(e) => {
                log::warn!(
                    "{} mmap failed, falling back to reading file, error: {}",
                    "Warning:".bright_yellow().bold(),
                    e
                );
                std::fs::read_to_string(input)
                    .map_err(|e| GtfSortError::IoError("reading input file", e))
                    .map(Cow::Owned)?
            }
        };

        #[cfg(not(feature = "mmap"))]
        let contents = std::fs::read_to_string(input)
            .map_err(|e| GtfSortError::IoError("reading input file", e))?;

        let contents_ref = contents.as_ref();

        let records = timed("Parsing input", Some(&mut ret.parsing_secs), || {
            match input_ext {
                "gff" | "gff3" => parallel_parse::<b'='>(contents_ref),
                "gtf" => parallel_parse::<b' '>(contents_ref),
                _ => Err("Unknown file extension, please specify a GTF or GFF3 file"),
            }
            .map_err(GtfSortError::ParseError)
        })?;

        let index = DashMap::<&str, Layers>::new();

        timed("building index", Some(&mut ret.indexing_secs), || {
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

        let mut writing_secs = 0.0;
        timed("Writing output", Some(&mut writing_secs), || {
            write_obj(
                output,
                &index,
                keys.iter()
                    .map(|chr| (*chr, index.get(chr).unwrap().count_line_size()))
                    .collect::<Vec<_>>(),
                &mut Some(&mut ret),
            )
        })
        .map_err(|e| GtfSortError::IoError("writing output file", e))?;
        ret.writing_secs = writing_secs;

        drop(records);
        drop(index);

        #[cfg(feature = "mmap")]
        if let Ok(m) = mmap_result {
            m.close()
                .map_err(|e| GtfSortError::IoError("syncing memory map", e))?;
        }

        ret.end_mem_mb = Some(max_mem_usage_mb());

        Ok(ret)
    })
}

pub fn sort_annotations_string<'a, const SEP: u8, OF: FnMut(&[u8]) -> io::Result<usize>>(
    input: &'a str,
    output: &mut OF,
    threads: usize,
) -> Result<SortAnnotationsJobResult<'a>, GtfSortError> {
    assert!(threads > 0, "Invalid number of threads");
    let mut ret = SortAnnotationsJobResult {
        input: "[string]",
        output: "[callback]",
        threads,
        input_mmaped: false,
        output_mmaped: false,
        parsing_secs: f64::NAN,
        indexing_secs: f64::NAN,
        writing_secs: f64::NAN,
        start_mem_mb: None,
        end_mem_mb: None,
    };

    let tp = rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .expect("Failed to build thread pool");

    let index = DashMap::<&str, Layers>::new();
    let keys = tp.install(|| {
        ret.start_mem_mb = Some(max_mem_usage_mb());

        let records = timed("Parsing input", Some(&mut ret.parsing_secs), || {
            parallel_parse::<SEP>(input).map_err(GtfSortError::ParseError)
        })?;

        timed("Building index", Some(&mut ret.indexing_secs), || {
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
            });
        });

        let mut keys: Vec<&str> = index.iter().map(|x| *x.key()).collect();
        keys.sort_by(|a, b| natord::compare(a, b));

        Ok(keys)
    })?;

    let mut writer = ChunkWriter::new(output);
    write_obj_sequential(
        &mut writer,
        &index,
        keys.iter()
            .map(|chr| (*chr, index.get(chr).unwrap().count_line_size()))
            .collect::<Vec<_>>(),
        &mut None,
    )
    .map_err(|e| GtfSortError::IoError("writing output file", e))?;

    ret.end_mem_mb = Some(max_mem_usage_mb());

    Ok(ret)
}
