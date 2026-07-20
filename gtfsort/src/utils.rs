use hashbrown::HashMap;
use rayon::prelude::*;

use colored::Colorize;

use dashmap::DashMap;
use flate2::{write::GzEncoder, Compression};
use std::cmp::Ordering;
use std::fmt::Debug;
use std::fs::File;
use std::io::{self, Write};
use std::path::Path;

use indoc::indoc;
use log::info;

use crate::gtf::Record;
use crate::SortAnnotationsJobResult;

const VERSION: &str = env!("CARGO_PKG_VERSION");

pub type Chrom<'a> = &'a str;
pub type ChromRecord<'a> = HashMap<Chrom<'a>, Vec<Record<'a>>>;

pub struct ChunkWriter<'f, F: FnMut(&[u8]) -> io::Result<usize>> {
    f: &'f mut F,
}

impl<'f, F: FnMut(&[u8]) -> io::Result<usize>> ChunkWriter<'f, F> {
    /// Wraps a byte callback in an [`io::Write`] implementation.
    pub fn new(f: &'f mut F) -> Self {
        Self { f }
    }
}

impl<F> Write for ChunkWriter<'_, F>
where
    F: FnMut(&[u8]) -> io::Result<usize>,
{
    /// Forwards a byte slice to the wrapped callback.
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        (self.f)(buf)
    }

    /// Completes immediately because callback-backed output has no local buffer.
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Runs an operation, records its elapsed seconds, and emits a timing log entry.
pub fn timed<T, F: FnOnce() -> T>(key: &str, output: Option<&mut f64>, f: F) -> T {
    let start = std::time::Instant::now();
    let res = f();
    let elapsed = start.elapsed().as_secs_f64();
    if let Some(output) = output {
        *output = elapsed;
    }
    log::info!("{}: {:.2}s", key, elapsed);
    res
}

/// Returns the leading blank and comment lines that form an annotation preamble.
///
/// Comments that appear after the first annotation record are intentionally not
/// included because only the file-level metadata header belongs at the top of a
/// sorted file.
pub(crate) fn leading_metadata_lines(input: &str) -> Vec<&str> {
    input
        .lines()
        .take_while(|line| {
            let trimmed = line.trim();
            trimmed.is_empty() || trimmed.starts_with('#')
        })
        .collect()
}

/// Counts the bytes needed to render metadata lines with trailing newlines.
#[cfg(feature = "mmap")]
fn metadata_size(metadata: &[&str]) -> usize {
    metadata.iter().map(|line| line.len() + 1).sum()
}

/// Writes metadata lines in their original order.
fn write_metadata<W: Write>(output: &mut W, metadata: &[&str]) -> Result<(), io::Error> {
    for line in metadata {
        writeln!(output, "{line}")?;
    }

    Ok(())
}

#[derive(Debug, Default)]
struct TranscriptAccumulator<'a> {
    transcript_lines: Vec<Record<'a>>,
    features: Vec<Record<'a>>,
}

#[derive(Debug, Default)]
struct GeneAccumulator<'a> {
    gene_lines: Vec<Record<'a>>,
    gene_level_features: Vec<Record<'a>>,
    transcripts: HashMap<&'a str, TranscriptAccumulator<'a>>,
}

#[derive(Debug)]
pub struct TranscriptBlock<'a> {
    transcript_id: &'a str,
    first_seen: usize,
    start: u32,
    transcript_lines: Vec<Record<'a>>,
    features: Vec<Record<'a>>,
}

#[derive(Debug)]
pub struct GeneBlock<'a> {
    gene_id: &'a str,
    first_seen: usize,
    start: u32,
    gene_lines: Vec<Record<'a>>,
    gene_level_features: Vec<Record<'a>>,
    transcripts: Vec<TranscriptBlock<'a>>,
}

#[derive(Debug, Default)]
pub struct Layers<'a> {
    pub genes: Vec<GeneBlock<'a>>,
}

impl<'a> Layers<'a> {
    /// Builds a sorted, write-ready chromosome view from parsed records.
    pub fn from_records(lines: &[Record<'a>]) -> Self {
        let mut genes = HashMap::<&'a str, GeneAccumulator<'a>>::new();

        for record in lines.iter().copied() {
            let gene = genes.entry(record.gene_id).or_default();

            if record.is_gene() {
                gene.gene_lines.push(record);
            } else if record.is_transcript() {
                gene.transcripts
                    .entry(record.transcript_id)
                    .or_default()
                    .transcript_lines
                    .push(record);
            } else if record.has_transcript() {
                gene.transcripts
                    .entry(record.transcript_id)
                    .or_default()
                    .features
                    .push(record);
            } else {
                gene.gene_level_features.push(record);
            }
        }

        let mut genes = genes
            .into_iter()
            .map(|(gene_id, acc)| GeneBlock::from_accumulator(gene_id, acc))
            .collect::<Vec<_>>();
        genes.sort_by(compare_gene_blocks);

        Self { genes }
    }

    /// Counts the rendered byte size for mmap-backed output allocation.
    pub fn count_line_size(&self) -> usize {
        let mut total = 0;
        self.for_each_line(|line| {
            total += line.len() + 1;
            Ok::<_, io::Error>(())
        })
        .expect("counting line sizes should not fail");
        total
    }

    /// Streams the chromosome contents in final output order.
    pub fn for_each_line<E, F>(&self, mut f: F) -> Result<(), E>
    where
        F: FnMut(&'a str) -> Result<(), E>,
    {
        for gene in &self.genes {
            gene.for_each_line(&mut f)?;
        }

        Ok(())
    }
}

impl<'a> GeneBlock<'a> {
    /// Finalizes a gene block by sorting gene lines, transcript blocks, and children.
    fn from_accumulator(gene_id: &'a str, mut acc: GeneAccumulator<'a>) -> Self {
        acc.gene_lines.sort_by(compare_position_then_line);
        acc.gene_level_features
            .sort_by(compare_position_then_feature_then_line);

        let mut transcripts = acc
            .transcripts
            .into_iter()
            .map(|(transcript_id, acc)| TranscriptBlock::from_accumulator(transcript_id, acc))
            .collect::<Vec<_>>();
        transcripts.sort_by(compare_transcript_blocks);

        let start = acc
            .gene_lines
            .iter()
            .chain(acc.gene_level_features.iter())
            .map(|record| record.start)
            .chain(transcripts.iter().map(|transcript| transcript.start))
            .min()
            .unwrap_or(0);

        Self {
            gene_id,
            first_seen: acc
                .gene_lines
                .iter()
                .chain(acc.gene_level_features.iter())
                .map(|record| record.line_no)
                .chain(transcripts.iter().map(|transcript| transcript.first_seen))
                .min()
                .unwrap_or(usize::MAX),
            start,
            gene_lines: acc.gene_lines,
            gene_level_features: acc.gene_level_features,
            transcripts,
        }
    }

    /// Visits each line in the order it should appear in the final file.
    fn for_each_line<E, F>(&self, f: &mut F) -> Result<(), E>
    where
        F: FnMut(&'a str) -> Result<(), E>,
    {
        for record in &self.gene_lines {
            f(record.line)?;
        }

        for record in &self.gene_level_features {
            f(record.line)?;
        }

        for transcript in &self.transcripts {
            transcript.for_each_line(f)?;
        }

        Ok(())
    }
}

impl<'a> TranscriptBlock<'a> {
    /// Finalizes a transcript block by sorting transcript rows and child features.
    fn from_accumulator(transcript_id: &'a str, mut acc: TranscriptAccumulator<'a>) -> Self {
        acc.transcript_lines.sort_by(compare_position_then_line);
        acc.features.sort_by(compare_transcript_features);

        let start = acc
            .transcript_lines
            .iter()
            .map(|record| record.start)
            .chain(acc.features.iter().map(|record| record.start))
            .min()
            .unwrap_or(0);

        Self {
            transcript_id,
            first_seen: acc
                .transcript_lines
                .iter()
                .chain(acc.features.iter())
                .map(|record| record.line_no)
                .min()
                .unwrap_or(usize::MAX),
            start,
            transcript_lines: acc.transcript_lines,
            features: acc.features,
        }
    }

    /// Visits each transcript line followed by its child features.
    fn for_each_line<E, F>(&self, f: &mut F) -> Result<(), E>
    where
        F: FnMut(&'a str) -> Result<(), E>,
    {
        for record in &self.transcript_lines {
            f(record.line)?;
        }

        for record in &self.features {
            f(record.line)?;
        }

        Ok(())
    }
}

/// Returns the effective annotation extension, ignoring an optional `.gz` suffix.
pub fn annotation_extension(path: &Path) -> Option<&str> {
    match path.extension()?.to_str()? {
        "gz" => Path::new(path.file_stem()?.to_str()?)
            .extension()
            .and_then(|ext| ext.to_str()),
        ext => Some(ext),
    }
}

/// Returns true when a path points to a gzip-compressed file.
pub fn is_gzip_path(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("gz"))
}

/// Orders genes by genomic start, input order, and natural gene identifier.
fn compare_gene_blocks(a: &GeneBlock<'_>, b: &GeneBlock<'_>) -> Ordering {
    a.start
        .cmp(&b.start)
        .then(a.first_seen.cmp(&b.first_seen))
        .then_with(|| natord::compare(a.gene_id, b.gene_id))
}

/// Orders transcripts deterministically by their first input occurrence and identifier.
fn compare_transcript_blocks(a: &TranscriptBlock<'_>, b: &TranscriptBlock<'_>) -> Ordering {
    a.first_seen
        .cmp(&b.first_seen)
        .then_with(|| natord::compare(a.transcript_id, b.transcript_id))
}

/// Orders records by coordinates and uses input order as a stable tie-breaker.
fn compare_position_then_line(a: &Record<'_>, b: &Record<'_>) -> Ordering {
    a.start
        .cmp(&b.start)
        .then(a.end.cmp(&b.end))
        .then(a.line_no.cmp(&b.line_no))
}

/// Orders gene-level features by coordinates, feature name, and input order.
fn compare_position_then_feature_then_line(a: &Record<'_>, b: &Record<'_>) -> Ordering {
    a.start
        .cmp(&b.start)
        .then(a.end.cmp(&b.end))
        .then_with(|| natord::compare(a.feat, b.feat))
        .then(a.line_no.cmp(&b.line_no))
}

/// Orders transcript children by exon grouping while retaining deterministic ties.
fn compare_transcript_features(a: &Record<'_>, b: &Record<'_>) -> Ordering {
    match (exon_group_rank(a.feat), exon_group_rank(b.feat)) {
        (Some(a_rank), Some(b_rank)) => natord::compare(a.exon_number, b.exon_number)
            .then(a_rank.cmp(&b_rank))
            .then(a.line_no.cmp(&b.line_no)),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => natord::compare(a.feat, b.feat).then(a.line_no.cmp(&b.line_no)),
    }
}

/// Maps exon-related feature names to their output rank within an exon group.
fn exon_group_rank(feat: &str) -> Option<u8> {
    match feat {
        "exon" => Some(0),
        "CDS" => Some(1),
        "start_codon" => Some(2),
        "stop_codon" => Some(3),
        _ => None,
    }
}

#[cfg(not(feature = "mmap"))]
#[inline(always)]
/// Writes sorted chromosome blocks to a file, selecting gzip output by suffix.
pub fn write_obj<'a, P: AsRef<Path> + Debug>(
    file: P,
    obj: &DashMap<&'a str, Layers>,
    keys: Vec<(&'a str, usize)>,
    job: &mut Option<&mut SortAnnotationsJobResult>,
) -> Result<(), io::Error> {
    write_obj_with_metadata(file, obj, keys, &[], job)
}

/// Writes sorted chromosome blocks and their leading metadata to a file.
#[cfg(not(feature = "mmap"))]
pub(crate) fn write_obj_with_metadata<'a, P: AsRef<Path> + Debug>(
    file: P,
    obj: &DashMap<&'a str, Layers>,
    keys: Vec<(&'a str, usize)>,
    metadata: &[&str],
    job: &mut Option<&mut SortAnnotationsJobResult>,
) -> Result<(), io::Error> {
    let path = file.as_ref();

    if is_gzip_path(path) {
        let f = open_output_file(path)?;
        let encoder = GzEncoder::new(f, Compression::default());
        write_obj_sequential_with_metadata(encoder, obj, keys, metadata, job)
    } else {
        let f = open_output_file(path)?;
        write_obj_sequential_with_metadata(f, obj, keys, metadata, job)
    }
}

#[cfg(feature = "mmap")]
#[inline(always)]
/// Writes sorted chromosome blocks to a file, preferring mmap for plain output.
pub fn write_obj<'a, P: AsRef<Path> + Debug>(
    file: P,
    obj: &DashMap<&'a str, Layers>,
    keys: Vec<(&'a str, usize)>,
    job: &mut Option<&mut SortAnnotationsJobResult>,
) -> Result<(), io::Error> {
    write_obj_with_metadata(file, obj, keys, &[], job)
}

/// Writes sorted chromosome blocks and their leading metadata to a file.
#[cfg(feature = "mmap")]
pub(crate) fn write_obj_with_metadata<'a, P: AsRef<Path> + Debug>(
    file: P,
    obj: &DashMap<&'a str, Layers>,
    keys: Vec<(&'a str, usize)>,
    metadata: &[&str],
    job: &mut Option<&mut SortAnnotationsJobResult>,
) -> Result<(), io::Error> {
    let path = file.as_ref();

    if is_gzip_path(path) {
        let f = open_output_file(path)?;
        let encoder = GzEncoder::new(f, Compression::default());
        return write_obj_sequential_with_metadata(encoder, obj, keys, metadata, job);
    }

    write_obj_mmaped_with_metadata(path, obj, keys.clone(), metadata, job).or_else(move |e| {
        log::warn!(
            "{} {}",
            "Error in mmaped output, falling back to sequential:"
                .bright_yellow()
                .bold(),
            e
        );

        let f = open_output_file(path)?;

        write_obj_sequential_with_metadata(f, obj, keys, metadata, job)
    })
}

/// Writes all chromosome blocks to a generic writer without using mmap.
pub fn write_obj_sequential<'a, W: Write>(
    file: W,
    obj: &DashMap<&'a str, Layers>,
    keys: Vec<(&'a str, usize)>,
    _job: &mut Option<&mut SortAnnotationsJobResult>,
) -> Result<(), io::Error> {
    write_obj_sequential_with_metadata(file, obj, keys, &[], _job)
}

/// Writes metadata and chromosome blocks to a generic writer without mmap.
pub(crate) fn write_obj_sequential_with_metadata<'a, W: Write>(
    file: W,
    obj: &DashMap<&'a str, Layers>,
    keys: Vec<(&'a str, usize)>,
    metadata: &[&str],
    _job: &mut Option<&mut SortAnnotationsJobResult>,
) -> Result<(), io::Error> {
    use std::io::BufWriter;

    let mut output = BufWriter::new(file);

    write_metadata(&mut output, metadata)?;
    write_layers(&mut output, obj, &keys)?;

    output.flush()?;

    Ok(())
}

#[cfg(feature = "mmap")]
/// Writes sorted chromosome blocks directly into a memory-mapped output file.
pub fn write_obj_mmaped<'a, P: AsRef<Path> + Debug>(
    file: P,
    obj: &DashMap<&'a str, Layers>,
    keys: Vec<(&'a str, usize)>,
    job: &mut Option<&mut SortAnnotationsJobResult>,
) -> Result<(), io::Error> {
    write_obj_mmaped_with_metadata(file, obj, keys, &[], job)
}

/// Writes metadata and sorted blocks directly into a memory-mapped output file.
#[cfg(feature = "mmap")]
fn write_obj_mmaped_with_metadata<'a, P: AsRef<Path> + Debug>(
    file: P,
    obj: &DashMap<&'a str, Layers>,
    keys: Vec<(&'a str, usize)>,
    metadata: &[&str],
    job: &mut Option<&mut SortAnnotationsJobResult>,
) -> Result<(), io::Error> {
    use std::{fs::OpenOptions, io::Cursor};

    use crate::mmap::{self, Madvice};

    let f = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(file)?;

    let size = metadata_size(metadata) as u64 + keys.iter().map(|(_, i)| *i as u64).sum::<u64>();

    if size == 0 {
        return Ok(());
    }

    f.set_len(size)?;

    #[cfg(unix)]
    let mut output_map = unsafe { mmap::MemoryMapMut::from_file(&f, size as usize)? };

    #[cfg(windows)]
    let mut output_map = unsafe { mmap::MemoryMapMut::from_handle(&f, size as usize)? };

    match output_map.madvise(&[Madvice::Random]) {
        Ok(_) => (),
        Err(e) => {
            log::warn!("{} {}", "Madvice error:".bright_yellow().bold(), e);
        }
    }

    let mut output = output_map.as_mut_slice();

    log::info!(
        "Successfully mapped output file, size: {} bytes",
        output.len()
    );

    let (metadata_output, remaining_output) = output.split_at_mut(metadata_size(metadata));
    let mut metadata_cursor = Cursor::new(metadata_output);
    write_metadata(&mut metadata_cursor, metadata)?;
    output = remaining_output;

    let mut output_slices = Vec::new();
    for (_, s) in keys.iter() {
        let (a, b) = output.split_at_mut(*s);
        output_slices.push(a);
        output = b;
    }

    keys.into_iter()
        .zip(output_slices)
        .collect::<Vec<_>>()
        .into_par_iter()
        .try_for_each(|((k, size_expected), output)| {
            let chr = obj.get(k).unwrap();

            let mut output = Cursor::new(output);
            chr.for_each_line(|line| writeln!(output, "{line}"))?;

            assert_eq!(
                output.position(),
                size_expected as u64,
                "Output buffer not empty, something went wrong"
            );

            Ok::<_, io::Error>(())
        })?;

    if let Some(j) = job.as_deref_mut() {
        j.output_mmaped = true;
    }

    output_map.close()?;

    Ok(())
}

/// Creates or truncates an output file and logs creation failures.
fn open_output_file(path: &Path) -> Result<File, io::Error> {
    File::create(path).map_err(|e| {
        log::error!("{} {}", "Error in output file:".bright_red().bold(), e);
        e
    })
}

/// Writes chromosome layers to a stream in the supplied chromosome order.
fn write_layers<'a, W: Write>(
    output: &mut W,
    obj: &DashMap<&'a str, Layers>,
    keys: &[(&'a str, usize)],
) -> Result<(), io::Error> {
    for &(k, _) in keys {
        let chr = obj.get(k).unwrap();
        chr.for_each_line(|line| writeln!(output, "{line}"))?;
    }

    Ok(())
}

#[derive(Default)]
struct ParseAccumulator<'a> {
    records: ChromRecord<'a>,
    error_count: usize,
    error_samples: Vec<String>,
}

impl<'a> ParseAccumulator<'a> {
    const MAX_ERROR_SAMPLES: usize = 5;

    /// Adds a parsed record to its chromosome bucket.
    fn push_record(&mut self, record: Record<'a>) {
        self.records.entry(record.chrom).or_default().push(record);
    }

    /// Counts a parse error and retains a bounded diagnostic sample.
    fn push_error(&mut self, error: String) {
        self.error_count += 1;
        if self.error_samples.len() < Self::MAX_ERROR_SAMPLES {
            self.error_samples.push(error);
        }
    }
}

/// Parses all non-comment annotation lines and surfaces parse failures explicitly.
pub fn parallel_parse<const SEP: u8>(s: &str) -> Result<ChromRecord<'_>, String> {
    let lines = s
        .lines()
        .enumerate()
        .filter(|(_, line)| {
            let trimmed = line.trim();
            !trimmed.is_empty() && !trimmed.starts_with('#')
        })
        .collect::<Vec<_>>();

    let parsed = lines
        .into_par_iter()
        .map(|(line_no, line)| Record::parse::<SEP>(line_no, line))
        .collect::<Vec<_>>();

    let mut acc = ParseAccumulator::default();
    for result in parsed {
        match result {
            Ok(record) => acc.push_record(record),
            Err(error) => acc.push_error(error.into_owned()),
        }
    }

    if acc.error_count > 0 {
        Err(format!(
            "failed to parse {} annotation line(s); sample errors: {}",
            acc.error_count,
            acc.error_samples.join(" | ")
        ))
    } else {
        Ok(acc.records)
    }
}

#[cfg(not(windows))]
/// Returns the process peak resident memory usage in mebibytes.
pub fn max_mem_usage_mb() -> f64 {
    let rusage = unsafe {
        let mut rusage = std::mem::MaybeUninit::uninit();
        if libc::getrusage(libc::RUSAGE_SELF, rusage.as_mut_ptr()) < 0 {
            info!("getrusage failed: {}", std::io::Error::last_os_error());
            return f64::NAN;
        }
        rusage.assume_init()
    };
    let maxrss = rusage.ru_maxrss as f64;
    if cfg!(target_os = "macos") {
        maxrss / 1024.0 / 1024.0
    } else {
        maxrss / 1024.0
    }
}

#[cfg(windows)]
/// Returns the process peak working-set size in mebibytes.
pub fn max_mem_usage_mb() -> f64 {
    use windows::Win32::System::{
        ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS},
        Threading::GetCurrentProcess,
    };

    unsafe {
        let h_proc = GetCurrentProcess();

        let mut pps = PROCESS_MEMORY_COUNTERS::default();
        if GetProcessMemoryInfo(
            h_proc,
            &mut pps,
            std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
        )
        .is_err()
        {
            info!(
                "GetProcessMemoryInfo failed: {}",
                std::io::Error::last_os_error()
            );
            return f64::NAN;
        }

        pps.PeakWorkingSetSize as f64 / 1024.0 / 1024.0
    }
}

/// Prints the command-line program banner and version.
pub fn msg() {
    println!(
        "{}\n{}\n{}",
        "\n##### GTFSORT #####".bright_purple().bold(),
        indoc!(
            "The fastest chr/pos/feature GTF/GFF sorter you'll see.
        Repo: github.com/alejandrogzi/gtfsort
        Feel free to contact the developer if any issue/bug is found.
        "
        ),
        format!("Version: {}", VERSION)
    );
}
