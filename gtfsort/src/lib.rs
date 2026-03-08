pub mod gtf;

pub use gtf::Record;

pub mod ord;
pub use ord::CowNaturalSort;

pub mod utils;
pub use utils::*;

pub mod interop;

#[cfg(feature = "testing")]
pub mod test_utils;
#[cfg(feature = "testing")]
pub use test_utils::*;

use std::{io, path::PathBuf};
use thiserror::Error;

use flate2::read::GzDecoder;
#[cfg(feature = "mmap")]
use mmap::Madvice;
#[cfg(feature = "mmap")]
use std::{borrow::Cow, fs::File};
use std::{io::Read, path::Path};

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
    ParseError(String),

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

#[derive(Clone, Copy)]
enum InputFormat {
    Gtf,
    Gff,
}

impl InputFormat {
    /// Detects the parser mode from a file path, allowing an optional `.gz` suffix.
    fn from_path(path: &Path) -> Result<Self, GtfSortError> {
        match annotation_extension(path) {
            Some("gtf") => Ok(Self::Gtf),
            Some("gff") | Some("gff3") => Ok(Self::Gff),
            Some(ext) => Err(GtfSortError::InvalidInput(format!(
                "unsupported annotation extension: {ext}"
            ))),
            None => Err(GtfSortError::InvalidInput(
                "Missing input file extension".to_string(),
            )),
        }
    }
}

/// Reads a text annotation file, transparently decompressing gzip input when needed.
fn read_annotation_to_string(path: &Path, gzip: bool) -> Result<String, GtfSortError> {
    if gzip {
        let file = std::fs::File::open(path)
            .map_err(|e| GtfSortError::IoError("opening input file", e))?;
        let mut decoder = GzDecoder::new(file);
        let mut contents = String::new();
        decoder
            .read_to_string(&mut contents)
            .map_err(|e| GtfSortError::IoError("decompressing input file", e))?;
        Ok(contents)
    } else {
        std::fs::read_to_string(path).map_err(|e| GtfSortError::IoError("reading input file", e))
    }
}

/// Parses the raw input contents using the format-specific attribute separator.
fn parse_records<'a>(input: &'a str, format: InputFormat) -> Result<ChromRecord<'a>, GtfSortError> {
    match format {
        InputFormat::Gtf => parallel_parse::<b' '>(input),
        InputFormat::Gff => parallel_parse::<b'='>(input),
    }
    .map_err(GtfSortError::ParseError)
}

/// Builds one sorted output block per chromosome.
fn build_index<'a>(records: &ChromRecord<'a>) -> DashMap<&'a str, Layers<'a>> {
    let index = DashMap::<&str, Layers>::new();

    records.par_iter().for_each(|(chrom, lines)| {
        index.insert(chrom, Layers::from_records(lines));
    });

    index
}

/// Returns chromosomes in natural sort order.
fn sorted_chrom_keys<'a>(index: &DashMap<&'a str, Layers<'a>>) -> Vec<&'a str> {
    let mut keys: Vec<&str> = index.iter().map(|entry| *entry.key()).collect();
    keys.sort_by(|a, b| natord::compare(a, b));
    keys
}

/// Computes per-chromosome output sizes for mmap-backed writes.
fn output_plan<'a>(
    index: &DashMap<&'a str, Layers<'a>>,
    keys: &[&'a str],
) -> Vec<(&'a str, usize)> {
    keys.iter()
        .map(|chrom| (*chrom, index.get(chrom).unwrap().count_line_size()))
        .collect()
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

    let input_format = InputFormat::from_path(input)?;
    let gzip_input = is_gzip_path(input);

    let tp = rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .expect("Failed to build thread pool");

    tp.install(|| {
        ret.start_mem_mb = Some(max_mem_usage_mb());

        log::info!("Using {} threads", threads);

        #[cfg(feature = "mmap")]
        let input_file = if gzip_input {
            None
        } else {
            Some(File::open(input).map_err(|e| GtfSortError::IoError("opening input file", e))?)
        };

        #[cfg(feature = "mmap")]
        let mmap_result = if gzip_input {
            None
        } else {
            let f = input_file.as_ref().unwrap();
            let f_size = f
                .metadata()
                .map_err(|e| GtfSortError::IoError("getting input file metadata", e))?
                .len();

            Some((|| {
                #[cfg(unix)]
                let contents_map = unsafe {
                    mmap::MemoryMap::<u8>::from_file(f, f_size as usize)
                        .map_err(|e| GtfSortError::IoError("mapping input file to memory", e))?
                };

                #[cfg(windows)]
                let contents_map = unsafe {
                    mmap::MemoryMap::<u8>::from_handle(&f, f_size as usize)
                        .map_err(|e| GtfSortError::IoError("mapping input file to memory", e))?
                };

                match contents_map.madvise(&[
                    Madvice::WillNeed,
                    Madvice::Sequential,
                    Madvice::HugePage,
                ]) {
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
            })())
        };

        #[cfg(feature = "mmap")]
        let contents = if gzip_input {
            Cow::Owned(read_annotation_to_string(input, true)?)
        } else {
            match mmap_result.as_ref().unwrap().as_ref() {
                Ok(m) => Cow::Borrowed(unsafe { std::str::from_utf8_unchecked(m.as_slice()) }),
                Err(e) => {
                    log::warn!(
                        "{} mmap failed, falling back to reading file, error: {}",
                        "Warning:".bright_yellow().bold(),
                        e
                    );
                    read_annotation_to_string(input, false).map(Cow::Owned)?
                }
            }
        };

        #[cfg(not(feature = "mmap"))]
        let contents = read_annotation_to_string(input, gzip_input)?;

        let contents_ref = contents.as_ref();

        let records = timed("Parsing input", Some(&mut ret.parsing_secs), || {
            parse_records(contents_ref, input_format)
        })?;

        let index = timed("building index", Some(&mut ret.indexing_secs), || {
            build_index(&records)
        });

        let keys = sorted_chrom_keys(&index);
        let plan = output_plan(&index, &keys);

        let mut writing_secs = 0.0;
        timed("Writing output", Some(&mut writing_secs), || {
            write_obj(output, &index, plan, &mut Some(&mut ret))
        })
        .map_err(|e| GtfSortError::IoError("writing output file", e))?;
        ret.writing_secs = writing_secs;

        drop(records);
        drop(index);

        #[cfg(feature = "mmap")]
        if let Some(Ok(m)) = mmap_result {
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

    let (index, keys) = tp.install(|| {
        ret.start_mem_mb = Some(max_mem_usage_mb());

        let records = timed("Parsing input", Some(&mut ret.parsing_secs), || {
            parallel_parse::<SEP>(input).map_err(GtfSortError::ParseError)
        })?;

        let built_index = timed("Building index", Some(&mut ret.indexing_secs), || {
            build_index(&records)
        });
        let keys = sorted_chrom_keys(&built_index);

        Ok((built_index, keys))
    })?;

    let plan = output_plan(&index, &keys);
    let mut writer = ChunkWriter::new(output);
    let mut writing_secs = 0.0;
    timed("Writing output", Some(&mut writing_secs), || {
        write_obj_sequential(&mut writer, &index, plan, &mut None)
    })
    .map_err(|e| GtfSortError::IoError("writing output file", e))?;
    ret.writing_secs = writing_secs;

    ret.end_mem_mb = Some(max_mem_usage_mb());

    Ok(ret)
}

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::{read::GzDecoder, write::GzEncoder, Compression};
    use std::fs;
    use std::io::Write;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn sort_string<const SEP: u8>(input: &str) -> Result<String, GtfSortError> {
        let mut output = Vec::new();
        sort_annotations_string::<SEP, _>(
            input,
            &mut |bytes| {
                output.extend_from_slice(bytes);
                Ok(bytes.len())
            },
            1,
        )?;

        Ok(String::from_utf8(output).unwrap())
    }

    fn temp_path(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        if let Some(stem) = name.strip_suffix(".gtf.gz") {
            std::env::temp_dir().join(format!("gtfsort_{stem}_{nanos}.gtf.gz"))
        } else if let Some(stem) = name.strip_suffix(".gff.gz") {
            std::env::temp_dir().join(format!("gtfsort_{stem}_{nanos}.gff.gz"))
        } else if let Some(stem) = name.strip_suffix(".gff3.gz") {
            std::env::temp_dir().join(format!("gtfsort_{stem}_{nanos}.gff3.gz"))
        } else {
            match name.rsplit_once('.') {
                Some((stem, ext)) => {
                    std::env::temp_dir().join(format!("gtfsort_{stem}_{nanos}.{ext}"))
                }
                None => std::env::temp_dir().join(format!("gtfsort_{name}_{nanos}")),
            }
        }
    }

    #[test]
    fn transcript_only_gtf_is_preserved() {
        let input = "\
chr1\tsrc\ttranscript\t100\t200\t.\t+\t.\tgene_id \"GENE1\"; transcript_id \"TX1\";\n\
chr1\tsrc\texon\t100\t150\t.\t+\t.\tgene_id \"GENE1\"; transcript_id \"TX1\"; exon_number \"1\";\n\
chr1\tsrc\texon\t180\t200\t.\t+\t.\tgene_id \"GENE1\"; transcript_id \"TX1\"; exon_number \"2\";\n";

        let output = sort_string::<b' '>(input).unwrap();

        assert_eq!(output, input);
    }

    #[test]
    fn xloc_gene_without_gene_line_is_not_dropped() {
        let input = "\
chr1\tsrc\tgene\t50\t300\t.\t+\t.\tgene_id \"ENSG1\";\n\
chr1\tsrc\ttranscript\t50\t300\t.\t+\t.\tgene_id \"ENSG1\"; transcript_id \"TXG1\";\n\
chr1\tsrc\texon\t50\t100\t.\t+\t.\tgene_id \"ENSG1\"; transcript_id \"TXG1\"; exon_number \"1\";\n\
chr1\tStringTie\ttranscript\t400\t500\t.\t+\t.\ttranscript_id \"TCONS_1\"; gene_id \"XLOC_1\"; gene_name \"XLOC_1\";\n\
chr1\tStringTie\texon\t400\t450\t.\t+\t.\ttranscript_id \"TCONS_1\"; gene_id \"XLOC_1\"; gene_name \"XLOC_1\"; exon_number \"1\";\n\
chr1\tStringTie\texon\t470\t500\t.\t+\t.\ttranscript_id \"TCONS_1\"; gene_id \"XLOC_1\"; gene_name \"XLOC_1\"; exon_number \"2\";\n";

        let output = sort_string::<b' '>(input).unwrap();

        assert!(output.contains("gene_id \"XLOC_1\""));
        assert_eq!(output.lines().count(), input.lines().count());
    }

    #[test]
    fn duplicate_child_features_are_preserved() {
        let input = "\
chr1\tsrc\tgene\t100\t300\t.\t+\t.\tgene_id \"GENE1\";\n\
chr1\tsrc\ttranscript\t100\t300\t.\t+\t.\tgene_id \"GENE1\"; transcript_id \"TX1\";\n\
chr1\tsrc\texon\t100\t200\t.\t+\t.\tgene_id \"GENE1\"; transcript_id \"TX1\"; exon_number \"1\";\n\
chr1\tsrc\tstart_codon\t100\t101\t.\t+\t0\tgene_id \"GENE1\"; transcript_id \"TX1\"; exon_number \"1\";\n\
chr1\tsrc\tstart_codon\t150\t150\t.\t+\t2\tgene_id \"GENE1\"; transcript_id \"TX1\"; exon_number \"1\";\n";

        let output = sort_string::<b' '>(input).unwrap();

        assert_eq!(output.matches("\tstart_codon\t").count(), 2);
    }

    #[test]
    fn parse_errors_are_reported() {
        let input = "\
chr1\tsrc\tgene\t1\t100\t.\t+\t.\tgene_id \"G1\";\n\
chr1\tsrc\ttranscript\t1\t100\t.\t+\t.\ttranscript_id \"TX1\";\n";

        let err = sort_string::<b' '>(input).unwrap_err();

        match err {
            GtfSortError::ParseError(message) => {
                assert!(message.contains("failed to parse"));
                assert!(message.contains("Missing gene_id"));
            }
            other => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn gzip_input_and_output_are_supported() {
        let input = "\
chr1\tsrc\ttranscript\t100\t200\t.\t+\t.\tgene_id \"GENE1\"; transcript_id \"TX1\";\n\
chr1\tsrc\texon\t100\t150\t.\t+\t.\tgene_id \"GENE1\"; transcript_id \"TX1\"; exon_number \"1\";\n";
        let input_path = temp_path("input.gtf.gz");
        let output_path = temp_path("output.gtf.gz");

        {
            let file = fs::File::create(&input_path).unwrap();
            let mut encoder = GzEncoder::new(file, Compression::default());
            encoder.write_all(input.as_bytes()).unwrap();
            encoder.finish().unwrap();
        }

        sort_annotations(&input_path, &output_path, 1).unwrap();

        let output = {
            let file = fs::File::open(&output_path).unwrap();
            let mut decoder = GzDecoder::new(file);
            let mut output = String::new();
            decoder.read_to_string(&mut output).unwrap();
            output
        };

        fs::remove_file(&input_path).unwrap();
        fs::remove_file(&output_path).unwrap();

        assert_eq!(output, input);
    }
}
