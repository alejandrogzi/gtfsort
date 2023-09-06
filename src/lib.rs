use std::collections::BTreeMap;
use std::io::{BufReader, BufRead, Write};
use std::fs::File;
use std::collections::HashMap;

use std::error::Error;

use natord::compare;

use log::Level;

use peak_alloc::PeakAlloc;

mod gtf;
use gtf::Record;

mod ord;
use ord::Sort;


#[global_allocator]
static PEAK_ALLOC: PeakAlloc = PeakAlloc;


pub fn gtfsort(input: &String, out: &String) -> Result<String, Box<dyn Error>> {

    simple_logger::init_with_level(Level::Info)?;

    let file = std::fs::File::open(input)?;
    let reader = BufReader::new(file);
    let mut output = File::create(out)?;

    let start = std::time::Instant::now();

    let mut layer: Vec<(String, i32, String, String)> = vec![];
    let mut mapper: HashMap<String, Vec<String>> = HashMap::new();
    let mut inner: HashMap<String, BTreeMap<Sort, String>> = HashMap::new();
    let mut helper: HashMap<String, String> = HashMap::new();

    for line in reader.lines() {
        let line = line?;

        if line.starts_with("#") {
            output.write_all(line.as_bytes())?;
            output.write_all(b"\n")?;
            continue;
        }

        let record = Record::new(line)?;

        match record.feature() {
            "gene" => {
                layer.push(record.outer_layer());
            }
            "transcript" => {
                let (gene, transcript, line) = record.gene_to_transcript();
                mapper.entry(gene).or_insert(Vec::new()).push(transcript.clone());
                helper.entry(transcript).or_insert(line);
            }
            _ => {
                let (transcript, exon_number, line) = record.inner_layer();
                inner.entry(transcript).or_insert(BTreeMap::new()).insert(Sort::new(exon_number.as_str()), line);
            },
        };
    }

    layer.sort_by(|a, b| {
        let cmp_chr = a.0.cmp(&b.0);
        if cmp_chr == std::cmp::Ordering::Equal {
            a.1.cmp(&b.1)
        } else {
            cmp_chr
        }
    });

    for i in layer {
        output.write_all(i.3.as_bytes())?;
        output.write_all(b"\n")?;
        let transcripts = mapper.get(&i.2).ok_or("Error: genes with 0 transcripts are not allowed")?;
        for j in transcripts {
            output.write_all(helper.get(j).unwrap().as_bytes())?;
            output.write_all(b"\n")?;
            let exons = inner.get(j).ok_or("Error: transcripts with 0 exons are not allowed")?;
            let joined_exons: String = exons.values().map(|value| value.to_string()).collect::<Vec<String>>().join("\n");
            output.write_all(joined_exons.as_bytes())?;
            output.write_all(b"\n")?;
        }
    }
    
    let elapsed = start.elapsed().as_secs_f32();
    let peak_mem = PEAK_ALLOC.peak_usage_as_mb();

    log::info!("Memory usage: {} MB", peak_mem);
    log::info!("Elapsed: {:.2?}", elapsed);

    Ok(out.to_string())
}


