use rayon::prelude::*;

use colored::Colorize;

use std::fs::File;
use std::io::{self, Read};
use std::path::PathBuf;

use indoc::indoc;

use crate::gtf::Record;

pub fn reader(file: &PathBuf) -> io::Result<String> {
    let mut file = File::open(file)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    Ok(contents)
}

pub fn parallel_parse<'a>(s: &'a str) -> Result<Vec<Record>, &'static str> {
    let records: Result<Vec<Record>, &'static str> =
        s.par_lines().map(|line| Record::parse(line)).collect();

    return records;
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
        "{}\n{}",
        "\n##### GTFSORT #####".bright_purple().bold(),
        indoc!(
            "A rapid chr/pos/feature gtf sorter in Rust.
        Repo: github.com/alejandrogzi/gtfsort
        "
        )
    );
}
