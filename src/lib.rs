use std::collections::{HashMap, BTreeMap};
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::error::Error;
use std::time::Instant;

use rayon::prelude::*;

use indoc::indoc;

use natord::compare;

mod gtf;
use gtf::Record;


fn gtf_line_parser(lines: Vec<String>) -> HashMap<String, BTreeMap<i32, HashMap<String, BTreeMap<String, BTreeMap<i32, String>>>>> {
    let mut chrom_dict: HashMap<String, BTreeMap<i32, HashMap<String, BTreeMap<String, BTreeMap<i32, String>>>>> = HashMap::new();

    for line in lines {
        if line.starts_with("#") {
            continue;
        } else {
            let gp = Record::new(&line);

            match gp.feat.as_str() {
                "gene" => {
                    let chrom_entry = chrom_dict.entry(gp.chrom.clone()).or_insert_with(BTreeMap::new);
                    let pos_entry = chrom_entry.entry(gp.pos).or_insert_with(HashMap::new);
                    let gene_entry = pos_entry.entry(gp.gene_id.clone()).or_insert_with(BTreeMap::new);
                    let transcript_entry = gene_entry.entry("00".to_string()).or_insert(BTreeMap::new());
                    transcript_entry.insert(0, line);
                }
                "transcript" => {
                    for (_, pos) in &mut chrom_dict {
                        for (_, gene) in pos {
                            for (g, transcript) in gene {
                                if gp.gene_id == *g {
                                    let transcript_entry = transcript.entry(gp.transcript_id.clone()).or_insert_with(BTreeMap::new);
                                    transcript_entry.insert(0, gp.line.clone());
                                }
                            }
                        }
                    }
                }
                _ => {
                    for (_, pos) in &mut chrom_dict {
                        for (_, gene) in pos {
                            for (g, transcript) in gene {
                                if gp.gene_id == *g {
                                    for (t, _) in &mut *transcript {
                                        if gp.transcript_id == *t {
                                            let k = match gp.feat.as_str() {
                                                "exon" => gp.exon_number.parse::<i32>().unwrap()*10,
                                                "CDS" => gp.exon_number.parse::<i32>().unwrap()*10+1,
                                                "5UTR" => 9998,
                                                "3UTR" => 9999, 
                                                "start_codon" => gp.exon_number.parse::<i32>().unwrap()*1000+4,
                                                "stop_codon" => gp.exon_number.parse::<i32>().unwrap()*1000+5,
                                                _ => 99999,
                                            };
                                            let transcript_entry = transcript.entry(gp.transcript_id.clone()).or_insert_with(BTreeMap::new);
                                            transcript_entry.insert(k, gp.line.clone());
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    chrom_dict
}



fn parallel_parse(file: File) -> HashMap<String, BTreeMap<i32, HashMap<String, BTreeMap<String, BTreeMap<i32, String>>>>> {
    
    let reader = BufReader::new(file);
    let lines: Vec<String> = reader.lines().map(|l| l.expect("Could not read line")).collect();

    let num_chunks = 16;
    let chunk_size = lines.len() / num_chunks;
    let chunks: Vec<Vec<String>> = lines.chunks(chunk_size).map(|chunk| chunk.to_vec()).collect();

    let jobs: Vec<HashMap<String, BTreeMap<i32, HashMap<String, BTreeMap<String, BTreeMap<i32, String>>>>>> = chunks
        .par_iter()
        .map(|chunk| gtf_line_parser(chunk.clone()))
        .collect();

    let mut temp_gtf = HashMap::new();
    for job in jobs {
        for (chrom, pos_dict) in job {
            let chrom_entry = temp_gtf.entry(chrom).or_insert_with(BTreeMap::new);
            for (pos, gene_dict) in pos_dict {
                let pos_entry = chrom_entry.entry(pos).or_insert_with(HashMap::new);
                pos_entry.extend(gene_dict);
            }
        }
    }
    temp_gtf
}




fn gtf_writter(tmp: HashMap<String, BTreeMap<i32, HashMap<String, BTreeMap<String, BTreeMap<i32, String>>>>>, output: &str) -> Result<(), Box<dyn Error>> {
    let mut output = File::create(output)?;

    let mut chromosomes = tmp.keys().cloned().collect::<Vec<String>>();
    chromosomes.sort_by(|a, b| compare(a, b));

    for chr in chromosomes {
        for (_, gene_dict) in tmp.get(&chr).unwrap() {
            for (_, transcript_dict) in gene_dict {
                for (_, exon_dict) in transcript_dict {
                    for (_, line) in exon_dict {
                        writeln!(output, "{}", line);
                    }
                }
            }
        }
    }
    Ok(())
}


pub fn gtfsort(input: &str, output: &str) {

    let start = Instant::now();
    
    println!("{}", indoc!(
        "\n
        ##### GTFSORT #####
        A rapid chr/pos/feature gtf sorter in Rust.\n
        Repo: https://github.com/alejandrogzi/gtfsort
        "));

    let num = num_cpus::get();
    println!("Number of threads: {}", num);

    let gtf_unsorted = File::open(input).unwrap();
    let gtf_sorted = parallel_parse(gtf_unsorted);
    gtf_writter(gtf_sorted, output).unwrap();

    let elapsed = start.elapsed();
    println!("Elapsed: {:.2?}", elapsed);
}