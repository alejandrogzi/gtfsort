use hashbrown::HashMap;
use rayon::prelude::*;

use colored::Colorize;

use dashmap::DashMap;
use std::collections::BTreeMap;
use std::fmt::Debug;
use std::fs::File;
use std::io::BufWriter;
use std::io::{self, Read, Write};
use std::path::Path;

use indoc::indoc;

use crate::gtf::Record;
use crate::ord::CowNaturalSort;

const VERSION: &str = env!("CARGO_PKG_VERSION");

pub type Chrom<'a> = &'a str;
pub type ChromRecord<'a> = HashMap<Chrom<'a>, Vec<Record<'a>>>;

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

pub fn reader<P: AsRef<Path> + Debug>(file: P) -> io::Result<String> {
    let mut file = File::open(file)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    Ok(contents)
}

pub fn write_obj<'a, P: AsRef<Path> + Debug>(
    file: P,
    obj: &DashMap<&'a str, Layers>,
    keys: &Vec<&'a str>,
) -> Result<(), io::Error> {
    let f = match File::create(file) {
        Ok(f) => f,
        Err(e) => {
            log::error!("{} {}", "Error in output file:".bright_red().bold(), e);
            std::process::exit(1);
        }
    };

    let mut output = BufWriter::new(f);

    for k in keys {
        let chr = obj.get(k).unwrap();

        for i in chr.layer.iter() {
            writeln!(output, "{}", i.2).unwrap();

            let transcripts = chr.mapper.get(&i.1).unwrap();
            for j in transcripts.iter() {
                writeln!(output, "{}", chr.helper.get(j).unwrap()).unwrap();
                let exons = chr.inner.get(j).unwrap();
                exons
                    .values()
                    .flatten()
                    .for_each(|x| writeln!(output, "{}", x).unwrap());
            }
        }
    }

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
