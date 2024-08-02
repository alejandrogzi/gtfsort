use hashbrown::HashMap;
use rayon::prelude::*;

use colored::Colorize;

use dashmap::DashMap;
use std::cell::UnsafeCell;
use std::collections::BTreeMap;
use std::fmt::Debug;
use std::io::{self, Write};
use std::ops::Deref;
use std::path::Path;

use indoc::indoc;

use crate::gtf::Record;
use crate::ord::CowNaturalSort;

const VERSION: &str = env!("CARGO_PKG_VERSION");

pub type Chrom<'a> = &'a str;
pub type ChromRecord<'a> = HashMap<Chrom<'a>, Vec<Record<'a>>>;

/// Polyfill foe `std::cell:SyncUnsafeCell`
pub struct SyncUnsafeCell<T>(UnsafeCell<T>);

unsafe impl<T> Sync for SyncUnsafeCell<T> {}

impl<T> Deref for SyncUnsafeCell<T> {
    type Target = UnsafeCell<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> SyncUnsafeCell<T> {
    pub fn new(value: T) -> Self {
        Self(UnsafeCell::new(value))
    }
    pub fn into_inner(self) -> T {
        self.0.into_inner()
    }
}

#[derive(Debug)]
pub struct Layers<'a> {
    // (start, gene_id, line)
    pub layer: Vec<(u32, &'a str, &'a str)>,
    // gene_id -> [transcript_id, transcript_id, ...]
    pub mapper: HashMap<&'a str, Vec<&'a str>>,
    // transcript_id -> {feat -> line}
    pub inner: HashMap<&'a str, BTreeMap<CowNaturalSort<'a>, Vec<&'a str>>>,
    // transcript_id -> line
    pub helper: HashMap<&'a str, &'a str>,
}

impl<'a> Layers<'a> {
    pub fn combine(&mut self, other: Layers<'a>) {
        self.layer.extend(other.layer);
        self.mapper.extend(other.mapper);
        self.inner.extend(other.inner);
        self.helper.extend(other.helper);
    }
    pub fn count_line_size(&self) -> usize {
        let mut total = 0;

        for i in self.layer.iter() {
            total += i.2.len() + 1;
            let transcripts = self.mapper.get(&i.1).unwrap();
            for j in transcripts.iter() {
                total += self.helper.get(j).unwrap().len() + 1;
                let exons = self.inner.get(j).unwrap();
                total += exons.values().flatten().map(|x| x.len() + 1).sum::<usize>();
            }
        }

        total
    }
}

impl<'a> Default for Layers<'a> {
    fn default() -> Self {
        Self {
            layer: Vec::new(),
            mapper: HashMap::new(),
            inner: HashMap::new(),
            helper: HashMap::new(),
        }
    }
}

#[cfg(not(feature = "mmap"))]
pub fn write_obj<'a, P: AsRef<Path> + Debug>(
    file: P,
    obj: &DashMap<&'a str, Layers>,
    keys: Vec<(&'a str, usize)>,
) -> Result<(), io::Error> {
    let f = match File::create(file) {
        Ok(f) => f,
        Err(e) => {
            log::error!("{} {}", "Error in output file:".bright_red().bold(), e);
            std::process::exit(1);
        }
    };

    let mut output = BufWriter::new(f);

    for (k, _) in keys {
        let chr = obj.get(k).unwrap();

        for i in chr.layer.iter() {
            writeln!(output, "{}", i.2)?;

            let transcripts = chr.mapper.get(&i.1).unwrap();
            for j in transcripts.iter() {
                writeln!(output, "{}", chr.helper.get(j).unwrap())?;
                let exons = chr.inner.get(j).unwrap();
                exons
                    .values()
                    .flatten()
                    .try_for_each(|x| writeln!(output, "{}", x))?;
            }
        }
    }

    Ok(())
}

#[cfg(feature = "mmap")]
pub fn write_obj<'a, P: AsRef<Path> + Debug>(
    file: P,
    obj: &DashMap<&'a str, Layers>,
    keys: Vec<(&'a str, usize)>,
) -> Result<(), io::Error> {
    use std::{fs::OpenOptions, io::Cursor};

    use crate::mmap::{self, Madvice};

    let f = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(file)?;

    let size = keys.iter().map(|(_, i)| *i as u64).sum();

    f.set_len(size)?;

    #[cfg(unix)]
    let mut output_map = unsafe { mmap::MemoryMapMut::from_file(&f, size as usize)? };

    #[cfg(windows)]
    let mut output_map = unsafe { mmap::MemoryMapMut::from_handle(&f, Some(size as usize))? };

    match output_map.madvise(&[Madvice::Random]) {
        Ok(_) => (),
        Err(e) => {
            log::warn!("{} {}", "Madvice error:".bright_yellow().bold(), e);
        }
    }

    let mut output = output_map.as_mut_slice();

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

            for i in chr.layer.iter() {
                writeln!(output, "{}", i.2)?;

                let transcripts = chr.mapper.get(&i.1).unwrap();
                for j in transcripts.iter() {
                    writeln!(output, "{}", chr.helper.get(j).unwrap())?;
                    let exons = chr.inner.get(j).unwrap();
                    exons
                        .values()
                        .flatten()
                        .try_for_each(|x| writeln!(output, "{}", x))?;
                }
            }

            assert_eq!(
                output.position(),
                size_expected as u64,
                "Output buffer not empty, something went wrong"
            );

            Ok::<_, io::Error>(())
        })?;

    Ok(())
}

pub fn parallel_parse<const SEP: u8>(s: &str) -> Result<ChromRecord<'_>, &'static str> {
    let x = s
        .par_lines()
        .filter(|line| !line.starts_with("#"))
        .filter_map(|line| Record::parse::<SEP>(line).ok())
        .fold(HashMap::new, |mut acc: ChromRecord, record| {
            acc.entry(record.chrom).or_default().push(record);
            acc
        })
        .reduce(HashMap::new, |mut acc, map| {
            for (k, v) in map {
                acc.entry(k).or_default().extend(v);
            }
            acc
        });

    Ok(x)
}

#[cfg(not(windows))]
pub fn max_mem_usage_mb() -> f64 {
    let rusage = unsafe {
        let mut rusage = std::mem::MaybeUninit::uninit();
        libc::getrusage(libc::RUSAGE_SELF, rusage.as_mut_ptr());
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
pub fn max_mem_usage_mb() -> f64 {
    use windows::Win32::System::{
        ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS},
        Threading::GetCurrentProcess,
    };

    unsafe {
        let h_proc = GetCurrentProcess();

        let mut pps = PROCESS_MEMORY_COUNTERS::default();
        GetProcessMemoryInfo(
            h_proc,
            &mut pps,
            std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32,
        )
        .ok();

        pps.PeakWorkingSetSize as f64 / 1024.0 / 1024.0
    }
}

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
