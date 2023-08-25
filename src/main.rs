use gtfsort::{gtfsort, benchmark};

use clap::{Arg, Command, ArgMatches};

use colored::Colorize;

use num_cpus;

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
        .arg(Arg::new("cpu")
            .index(3)
            .required(false)
            .default_value("max")
            .value_name("NUM_CPUS")
            .help("Number of cpus to use"))
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
    let cpu: &String = matches.get_one("cpu").unwrap();
    let max = num_cpus::get();

    let n = match cpu.as_str() {
        "max" => {
            max
        }
        _ => {
            if cpu.parse::<usize>().is_err() {
                eprintln!("{} {}",
                        "Error:".bright_red().bold(),
                        "Number of cpus must be an integer".bright_red());
                std::process::exit(1);
            } else {
                if cpu.parse::<usize>().unwrap() > max {
                    eprintln!("{} {} {} {} {}",
                            "Error:".bright_red().bold(),
                            "Number of cpus must be less than or equal to".bright_red(),
                            max.to_string().bright_red(),
                            "\n Gtfsort will use:".bright_red().bold(),
                            max.to_string().bright_red());
                    max
                } else {
                    cpu.parse::<usize>().unwrap()
                }
            }
        }
    };

    let _ = gtfsort(i, o, n);
    // benchmark();

    Ok(())
}

