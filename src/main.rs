use gtfsort::gtfsort;

use clap::{Arg, ArgMatches, Command};

use colored::Colorize;

use std::error::Error;
use std::string::String;

fn main() {
    let matches = Command::new("gtfsort")
        .version("0.1.1")
        .author("Alejandro Gonzales-Irribarren <jose.gonzalesdezavala1@unmsm.edu.pe>")
        .about("An optimized chr/pos/feature GTF2.5-3 sorter using a lexicographic-based index ordering algorithm written in Rust.")
        .arg(Arg::new("i")
            .index(1)
            .required(true)
            .value_name("GTF")
            .help("GTF file to sort"))
        .arg(Arg::new("o")
            .index(2)
            .required(true)
            .value_name("OUTPUT")
            .help("Output sorted gtf file"))
        .get_matches();

    if let Some(err) = run(matches).err() {
        eprintln!("{} {}", "Error:".bright_red().bold(), err);
        std::process::exit(1);
    }
}

fn run(matches: ArgMatches) -> Result<(), Box<dyn Error>> {
    let i: &String = matches.get_one("i").unwrap();
    let o: &String = matches.get_one("o").unwrap();

    let _ = gtfsort(i, o);

    println!(
        "{} {}",
        "Success:".bright_green().bold(),
        "GTF file sorted successfully!"
    );

    Ok(())
}
