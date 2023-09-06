use gtfsort::gtfsort;

use clap::{Arg, Command, ArgMatches};

use colored::Colorize;

use std::string::String;
use std::error::Error;


fn main() {
    let matches = Command::new("gtfsort")
        .version("0.1.0")
        .author("Alejandro Gonzales-Irribarren <jose.gonzalesdezavala1@unmsm.edu.pe>")
        .about("...")
        .arg(Arg::new("i")
            .index(1)
            .required(true)
            .value_name("GTF")
            .help("GTF file to sort"))
        .arg(Arg::new("o")
            .index(2)
            .required(true)
            .value_name("OUTPUT")
            .help("Output gtf file"))
        .get_matches();

    if let Some(err) = run(matches).err() {
        eprintln!("{} {}", 
                "Error:".bright_red().bold(),
                err);
        std::process::exit(1);
    }
}


fn run(matches: ArgMatches) -> Result<(), Box<dyn Error>> {
    let i: &String = matches.get_one("i").unwrap();
    let o: &String = matches.get_one("o").unwrap();

    let _ = gtfsort(i, o);

    Ok(())
}