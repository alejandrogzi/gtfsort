use std::collections::BTreeMap;
use std::io::{BufReader, BufRead, Write};
use std::fs::File;
use std::collections::HashMap;

use std::error::Error;

use log::Level;

use indoc::indoc;

use peak_alloc::PeakAlloc;

mod gtf;
use gtf::Record;

mod ord;
use ord::Sort;


#[global_allocator]
static PEAK_ALLOC: PeakAlloc = PeakAlloc;


pub fn gtfsort(input: &String, out: &String) -> Result<String, Box<dyn Error>> {

    if std::fs::metadata(input)?.len() == 0{
        Err("Error: input file is empty")?;
    }

    msg();
    simple_logger::init_with_level(Level::Info)?;

    let file = std::fs::File::open(input)?;
    let reader = BufReader::new(file);
    let mut output = File::create(out)?;

    let start = std::time::Instant::now();

    let mut layer: Vec<(String, i32, String, String)> = vec![];
    let mut mapper: HashMap<String, Vec<String>> = HashMap::new();
    let mut inner: HashMap<String, BTreeMap<Sort, String>> = HashMap::new();
    let mut helper: HashMap<String, String> = HashMap::new();

    log::info!("Sorting...");

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

    let mut track = true; 

    for i in layer {
        if !track {
            output.write_all(b"\n")?;
        } else {
            track = false; 
        }
    
        output.write_all(i.3.as_bytes())?;
        output.write_all(b"\n")?;
        let transcripts = mapper.get(&i.2).ok_or("Error: genes with 0 transcripts are not allowed")?;
        for (index, j) in transcripts.iter().enumerate() {
            output.write_all(helper.get(j).unwrap().as_bytes())?;
            output.write_all(b"\n")?;
            let exons = inner.get(j).ok_or("Error: transcripts with 0 exons are not allowed")?;
            let joined_exons: String = exons.values().map(|value| value.to_string()).collect::<Vec<String>>().join("\n");
            output.write_all(joined_exons.as_bytes())?;
            if index < transcripts.len() - 1 {
                output.write_all(b"\n")?;
            }
        }
    }
    

    let elapsed = start.elapsed().as_secs_f32();
    let peak_mem = PEAK_ALLOC.peak_usage_as_mb();

    log::info!("Memory usage: {} MB", peak_mem);
    log::info!("Elapsed: {:.2?}", elapsed);

    Ok(out.to_string())
}



fn msg() {
    println!("{}", indoc!(
        "\n
        ##### GTFSORT #####
        A rapid chr/pos/feature gtf sorter in Rust.
        Repo: https://github.com/alejandrogzi/gtfsort
        "));
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    
    fn create_test_file(content: &str) -> (String, String) {
        let ifile = "test_input.gtf";
        let ofile = "test_output.gtf";

        let mut input_file = File::create(ifile).unwrap();
        input_file.write_all(content.as_bytes()).unwrap();

        (ifile.to_string() , ofile.to_string())
    }

    #[test]
    fn test_gtfsort_inner_order() {
        let input_content = indoc!(
            "1\tensembl_havana\tCDS\t7217861\t7217963\t.\t+\t2\tgene_id \"ENSMUSG00000051285\"; gene_version \"18\"; transcript_id \"ENSMUST00000061280\"; transcript_version \"17\"; exon_number \"3\"; gene_name \"Pcmtd1\"; gene_source \"ensembl_havana\"; gene_biotype \"protein_coding\"; transcript_name \"Pcmtd1-201\"; transcript_source \"ensembl_havana\"; transcript_biotype \"protein_coding\"; tag \"CCDS\"; ccds_id \"CCDS35508\"; protein_id \"ENSMUSP00000059261\"; protein_version \"10\"; tag \"basic\"; tag \"Ensembl_canonical\"; transcript_support_level \"1 (assigned to previous version 16)\";
            1\tensembl_havana\texon\t7231116\t7231287\t.\t+\t.\tgene_id \"ENSMUSG00000051285\"; gene_version \"18\"; transcript_id \"ENSMUST00000061280\"; transcript_version \"17\"; exon_number \"4\"; gene_name \"Pcmtd1\"; gene_source \"ensembl_havana\"; gene_biotype \"protein_coding\"; transcript_name \"Pcmtd1-201\"; transcript_source \"ensembl_havana\"; transcript_biotype \"protein_coding\"; tag \"CCDS\"; ccds_id \"CCDS35508\"; exon_id \"ENSMUSE00001268642\"; exon_version \"2\"; tag \"basic\"; tag \"Ensembl_canonical\"; transcript_support_level \"1 (assigned to previous version 16)\";
            1\tensembl_havana\tCDS\t7231116\t7231287\t.\t+\t1\tgene_id \"ENSMUSG00000051285\"; gene_version \"18\"; transcript_id \"ENSMUST00000061280\"; transcript_version \"17\"; exon_number \"4\"; gene_name \"Pcmtd1\"; gene_source \"ensembl_havana\"; gene_biotype \"protein_coding\"; transcript_name \"Pcmtd1-201\"; transcript_source \"ensembl_havana\"; transcript_biotype \"protein_coding\"; tag \"CCDS\"; ccds_id \"CCDS35508\"; protein_id \"ENSMUSP00000059261\"; protein_version \"10\"; tag \"basic\"; tag \"Ensembl_canonical\"; transcript_support_level \"1 (assigned to previous version 16)\";
            1\tensembl_havana\texon\t7233472\t7233595\t.\t+\t.\tgene_id \"ENSMUSG00000051285\"; gene_version \"18\"; transcript_id \"ENSMUST00000061280\"; transcript_version \"17\"; exon_number \"5\"; gene_name \"Pcmtd1\"; gene_source \"ensembl_havana\"; gene_biotype \"protein_coding\"; transcript_name \"Pcmtd1-201\"; transcript_source \"ensembl_havana\"; transcript_biotype \"protein_coding\"; tag \"CCDS\"; ccds_id \"CCDS35508\"; exon_id \"ENSMUSE00001298878\"; exon_version \"2\"; tag \"basic\"; tag \"Ensembl_canonical\"; transcript_support_level \"1 (assigned to previous version 16)\";
            1\tensembl_havana\tCDS\t7233472\t7233595\t.\t+\t0\tgene_id \"ENSMUSG00000051285\"; gene_version \"18\"; transcript_id \"ENSMUST00000061280\"; transcript_version \"17\"; exon_number \"5\"; gene_name \"Pcmtd1\"; gene_source \"ensembl_havana\"; gene_biotype \"protein_coding\"; transcript_name \"Pcmtd1-201\"; transcript_source \"ensembl_havana\"; transcript_biotype \"protein_coding\"; tag \"CCDS\"; ccds_id \"CCDS35508\"; protein_id \"ENSMUSP00000059261\"; protein_version \"10\"; tag \"basic\"; tag \"Ensembl_canonical\"; transcript_support_level \"1 (assigned to previous version 16)\";
            1\tensembl_havana\texon\t7239739\t7243852\t.\t+\t.\tgene_id \"ENSMUSG00000051285\"; gene_version \"18\"; transcript_id \"ENSMUST00000061280\"; transcript_version \"17\"; exon_number \"6\"; gene_name \"Pcmtd1\"; gene_source \"ensembl_havana\"; gene_biotype \"protein_coding\"; transcript_name \"Pcmtd1-201\"; transcript_source \"ensembl_havana\"; transcript_biotype \"protein_coding\"; tag \"CCDS\"; ccds_id \"CCDS35508\"; exon_id \"ENSMUSE00000466061\"; exon_version \"7\"; tag \"basic\"; tag \"Ensembl_canonical\"; transcript_support_level \"1 (assigned to previous version 16)\";
            1\tensembl_havana\tCDS\t7239739\t7240103\t.\t+\t2\tgene_id \"ENSMUSG00000051285\"; gene_version \"18\"; transcript_id \"ENSMUST00000061280\"; transcript_version \"17\"; exon_number \"6\"; gene_name \"Pcmtd1\"; gene_source \"ensembl_havana\"; gene_biotype \"protein_coding\"; transcript_name \"Pcmtd1-201\"; transcript_source \"ensembl_havana\"; transcript_biotype \"protein_coding\"; tag \"CCDS\"; ccds_id \"CCDS35508\"; protein_id \"ENSMUSP00000059261\"; protein_version \"10\"; tag \"basic\"; tag \"Ensembl_canonical\"; transcript_support_level \"1 (assigned to previous version 16)\";
            1\tensembl_havana\tstop_codon\t7240104\t7240106\t.\t+\t0\tgene_id \"ENSMUSG00000051285\"; gene_version \"18\"; transcript_id \"ENSMUST00000061280\"; transcript_version \"17\"; exon_number \"6\"; gene_name \"Pcmtd1\"; gene_source \"ensembl_havana\"; gene_biotype \"protein_coding\"; transcript_name \"Pcmtd1-201\"; transcript_source \"ensembl_havana\"; transcript_biotype \"protein_coding\"; tag \"CCDS\"; ccds_id \"CCDS35508\"; tag \"basic\"; tag \"Ensembl_canonical\"; transcript_support_level \"1 (assigned to previous version 16)\";
            1\tensembl_havana\tCDS\t7190533\t7190839\t.\t+\t0\tgene_id \"ENSMUSG00000051285\"; gene_version \"18\"; transcript_id \"ENSMUST00000061280\"; transcript_version \"17\"; exon_number \"2\"; gene_name \"Pcmtd1\"; gene_source \"ensembl_havana\"; gene_biotype \"protein_coding\"; transcript_name \"Pcmtd1-201\"; transcript_source \"ensembl_havana\"; transcript_biotype \"protein_coding\"; tag \"CCDS\"; ccds_id \"CCDS35508\"; protein_id \"ENSMUSP00000059261\"; protein_version \"10\"; tag \"basic\"; tag \"Ensembl_canonical\"; transcript_support_level \"1 (assigned to previous version 16)\";
            1\tensembl_havana\tstart_codon\t7190533\t7190535\t.\t+\t0\tgene_id \"ENSMUSG00000051285\"; gene_version \"18\"; transcript_id \"ENSMUST00000061280\"; transcript_version \"17\"; exon_number \"2\"; gene_name \"Pcmtd1\"; gene_source \"ensembl_havana\"; gene_biotype \"protein_coding\"; transcript_name \"Pcmtd1-201\"; transcript_source \"ensembl_havana\"; transcript_biotype \"protein_coding\"; tag \"CCDS\"; ccds_id \"CCDS35508\"; tag \"basic\"; tag \"Ensembl_canonical\"; transcript_support_level \"1 (assigned to previous version 16)\";
            1\tensembl_havana\texon\t7217861\t7217963\t.\t+\t.\tgene_id \"ENSMUSG00000051285\"; gene_version \"18\"; transcript_id \"ENSMUST00000061280\"; transcript_version \"17\"; exon_number \"3\"; gene_name \"Pcmtd1\"; gene_source \"ensembl_havana\"; gene_biotype \"protein_coding\"; transcript_name \"Pcmtd1-201\"; transcript_source \"ensembl_havana\"; transcript_biotype \"protein_coding\"; tag \"CCDS\"; ccds_id \"CCDS35508\"; exon_id \"ENSMUSE00001273110\"; exon_version \"2\"; tag \"basic\"; tag \"Ensembl_canonical\"; transcript_support_level \"1 (assigned to previous version 16)\";
            1\tensembl_havana\tfive_prime_utr\t7159144\t7159440\t.\t+\t.\tgene_id \"ENSMUSG00000051285\"; gene_version \"18\"; transcript_id \"ENSMUST00000061280\"; transcript_version \"17\"; gene_name \"Pcmtd1\"; gene_source \"ensembl_havana\"; gene_biotype \"protein_coding\"; transcript_name \"Pcmtd1-201\"; transcript_source \"ensembl_havana\"; transcript_biotype \"protein_coding\"; tag \"CCDS\"; ccds_id \"CCDS35508\"; tag \"basic\"; tag \"Ensembl_canonical\"; transcript_support_level \"1 (assigned to previous version 16)\";
            1\tensembl_havana\tgene\t7159144\t7243852\t.\t+\t.\tgene_id \"ENSMUSG00000051285\"; gene_version \"18\"; gene_name \"Pcmtd1\"; gene_source \"ensembl_havana\"; gene_biotype \"protein_coding\";
            1\tensembl_havana\ttranscript\t7159144\t7243852\t.\t+\t.\tgene_id \"ENSMUSG00000051285\"; gene_version \"18\"; transcript_id \"ENSMUST00000061280\"; transcript_version \"17\"; gene_name \"Pcmtd1\"; gene_source \"ensembl_havana\"; gene_biotype \"protein_coding\"; transcript_name \"Pcmtd1-201\"; transcript_source \"ensembl_havana\"; transcript_biotype \"protein_coding\"; tag \"CCDS\"; ccds_id \"CCDS35508\"; tag \"basic\"; tag \"Ensembl_canonical\"; transcript_support_level \"1 (assigned to previous version 16)\";
            1\tensembl_havana\texon\t7159144\t7159440\t.\t+\t.\tgene_id \"ENSMUSG00000051285\"; gene_version \"18\"; transcript_id \"ENSMUST00000061280\"; transcript_version \"17\"; exon_number \"1\"; gene_name \"Pcmtd1\"; gene_source \"ensembl_havana\"; gene_biotype \"protein_coding\"; transcript_name \"Pcmtd1-201\"; transcript_source \"ensembl_havana\"; transcript_biotype \"protein_coding\"; tag \"CCDS\"; ccds_id \"CCDS35508\"; exon_id \"ENSMUSE00000630850\"; exon_version \"4\"; tag \"basic\"; tag \"Ensembl_canonical\"; transcript_support_level \"1 (assigned to previous version 16)\";
            1\tensembl_havana\texon\t7190418\t7190839\t.\t+\t.\tgene_id \"ENSMUSG00000051285\"; gene_version \"18\"; transcript_id \"ENSMUST00000061280\"; transcript_version \"17\"; exon_number \"2\"; gene_name \"Pcmtd1\"; gene_source \"ensembl_havana\"; gene_biotype \"protein_coding\"; transcript_name \"Pcmtd1-201\"; transcript_source \"ensembl_havana\"; transcript_biotype \"protein_coding\"; tag \"CCDS\"; ccds_id \"CCDS35508\"; exon_id \"ENSMUSE00000553965\"; exon_version \"3\"; tag \"basic\"; tag \"Ensembl_canonical\"; transcript_support_level \"1 (assigned to previous version 16)\";
            1\tensembl_havana\tthree_prime_utr\t7240107\t7243852\t.\t+\t.\tgene_id \"ENSMUSG00000051285\"; gene_version \"18\"; transcript_id \"ENSMUST00000061280\"; transcript_version \"17\"; gene_name \"Pcmtd1\"; gene_source \"ensembl_havana\"; gene_biotype \"protein_coding\"; transcript_name \"Pcmtd1-201\"; transcript_source \"ensembl_havana\"; transcript_biotype \"protein_coding\"; tag \"CCDS\"; ccds_id \"CCDS35508\"; tag \"basic\"; tag \"Ensembl_canonical\"; transcript_support_level \"1 (assigned to previous version 16)\";"
        );
        

        let (input_file, output_file) = create_test_file(input_content);
        let result = gtfsort(&input_file, &output_file);

        assert!(result.is_ok());

        let sorted_content = indoc!(
            "1\tensembl_havana\tgene\t7159144\t7243852\t.\t+\t.\tgene_id \"ENSMUSG00000051285\"; gene_version \"18\"; gene_name \"Pcmtd1\"; gene_source \"ensembl_havana\"; gene_biotype \"protein_coding\";
            1\tensembl_havana\ttranscript\t7159144\t7243852\t.\t+\t.\tgene_id \"ENSMUSG00000051285\"; gene_version \"18\"; transcript_id \"ENSMUST00000061280\"; transcript_version \"17\"; gene_name \"Pcmtd1\"; gene_source \"ensembl_havana\"; gene_biotype \"protein_coding\"; transcript_name \"Pcmtd1-201\"; transcript_source \"ensembl_havana\"; transcript_biotype \"protein_coding\"; tag \"CCDS\"; ccds_id \"CCDS35508\"; tag \"basic\"; tag \"Ensembl_canonical\"; transcript_support_level \"1 (assigned to previous version 16)\";
            1\tensembl_havana\texon\t7159144\t7159440\t.\t+\t.\tgene_id \"ENSMUSG00000051285\"; gene_version \"18\"; transcript_id \"ENSMUST00000061280\"; transcript_version \"17\"; exon_number \"1\"; gene_name \"Pcmtd1\"; gene_source \"ensembl_havana\"; gene_biotype \"protein_coding\"; transcript_name \"Pcmtd1-201\"; transcript_source \"ensembl_havana\"; transcript_biotype \"protein_coding\"; tag \"CCDS\"; ccds_id \"CCDS35508\"; exon_id \"ENSMUSE00000630850\"; exon_version \"4\"; tag \"basic\"; tag \"Ensembl_canonical\"; transcript_support_level \"1 (assigned to previous version 16)\";
            1\tensembl_havana\texon\t7190418\t7190839\t.\t+\t.\tgene_id \"ENSMUSG00000051285\"; gene_version \"18\"; transcript_id \"ENSMUST00000061280\"; transcript_version \"17\"; exon_number \"2\"; gene_name \"Pcmtd1\"; gene_source \"ensembl_havana\"; gene_biotype \"protein_coding\"; transcript_name \"Pcmtd1-201\"; transcript_source \"ensembl_havana\"; transcript_biotype \"protein_coding\"; tag \"CCDS\"; ccds_id \"CCDS35508\"; exon_id \"ENSMUSE00000553965\"; exon_version \"3\"; tag \"basic\"; tag \"Ensembl_canonical\"; transcript_support_level \"1 (assigned to previous version 16)\";
            1\tensembl_havana\tCDS\t7190533\t7190839\t.\t+\t0\tgene_id \"ENSMUSG00000051285\"; gene_version \"18\"; transcript_id \"ENSMUST00000061280\"; transcript_version \"17\"; exon_number \"2\"; gene_name \"Pcmtd1\"; gene_source \"ensembl_havana\"; gene_biotype \"protein_coding\"; transcript_name \"Pcmtd1-201\"; transcript_source \"ensembl_havana\"; transcript_biotype \"protein_coding\"; tag \"CCDS\"; ccds_id \"CCDS35508\"; protein_id \"ENSMUSP00000059261\"; protein_version \"10\"; tag \"basic\"; tag \"Ensembl_canonical\"; transcript_support_level \"1 (assigned to previous version 16)\";
            1\tensembl_havana\tstart_codon\t7190533\t7190535\t.\t+\t0\tgene_id \"ENSMUSG00000051285\"; gene_version \"18\"; transcript_id \"ENSMUST00000061280\"; transcript_version \"17\"; exon_number \"2\"; gene_name \"Pcmtd1\"; gene_source \"ensembl_havana\"; gene_biotype \"protein_coding\"; transcript_name \"Pcmtd1-201\"; transcript_source \"ensembl_havana\"; transcript_biotype \"protein_coding\"; tag \"CCDS\"; ccds_id \"CCDS35508\"; tag \"basic\"; tag \"Ensembl_canonical\"; transcript_support_level \"1 (assigned to previous version 16)\";
            1\tensembl_havana\texon\t7217861\t7217963\t.\t+\t.\tgene_id \"ENSMUSG00000051285\"; gene_version \"18\"; transcript_id \"ENSMUST00000061280\"; transcript_version \"17\"; exon_number \"3\"; gene_name \"Pcmtd1\"; gene_source \"ensembl_havana\"; gene_biotype \"protein_coding\"; transcript_name \"Pcmtd1-201\"; transcript_source \"ensembl_havana\"; transcript_biotype \"protein_coding\"; tag \"CCDS\"; ccds_id \"CCDS35508\"; exon_id \"ENSMUSE00001273110\"; exon_version \"2\"; tag \"basic\"; tag \"Ensembl_canonical\"; transcript_support_level \"1 (assigned to previous version 16)\";
            1\tensembl_havana\tCDS\t7217861\t7217963\t.\t+\t2\tgene_id \"ENSMUSG00000051285\"; gene_version \"18\"; transcript_id \"ENSMUST00000061280\"; transcript_version \"17\"; exon_number \"3\"; gene_name \"Pcmtd1\"; gene_source \"ensembl_havana\"; gene_biotype \"protein_coding\"; transcript_name \"Pcmtd1-201\"; transcript_source \"ensembl_havana\"; transcript_biotype \"protein_coding\"; tag \"CCDS\"; ccds_id \"CCDS35508\"; protein_id \"ENSMUSP00000059261\"; protein_version \"10\"; tag \"basic\"; tag \"Ensembl_canonical\"; transcript_support_level \"1 (assigned to previous version 16)\";
            1\tensembl_havana\texon\t7231116\t7231287\t.\t+\t.\tgene_id \"ENSMUSG00000051285\"; gene_version \"18\"; transcript_id \"ENSMUST00000061280\"; transcript_version \"17\"; exon_number \"4\"; gene_name \"Pcmtd1\"; gene_source \"ensembl_havana\"; gene_biotype \"protein_coding\"; transcript_name \"Pcmtd1-201\"; transcript_source \"ensembl_havana\"; transcript_biotype \"protein_coding\"; tag \"CCDS\"; ccds_id \"CCDS35508\"; exon_id \"ENSMUSE00001268642\"; exon_version \"2\"; tag \"basic\"; tag \"Ensembl_canonical\"; transcript_support_level \"1 (assigned to previous version 16)\";
            1\tensembl_havana\tCDS\t7231116\t7231287\t.\t+\t1\tgene_id \"ENSMUSG00000051285\"; gene_version \"18\"; transcript_id \"ENSMUST00000061280\"; transcript_version \"17\"; exon_number \"4\"; gene_name \"Pcmtd1\"; gene_source \"ensembl_havana\"; gene_biotype \"protein_coding\"; transcript_name \"Pcmtd1-201\"; transcript_source \"ensembl_havana\"; transcript_biotype \"protein_coding\"; tag \"CCDS\"; ccds_id \"CCDS35508\"; protein_id \"ENSMUSP00000059261\"; protein_version \"10\"; tag \"basic\"; tag \"Ensembl_canonical\"; transcript_support_level \"1 (assigned to previous version 16)\";
            1\tensembl_havana\texon\t7233472\t7233595\t.\t+\t.\tgene_id \"ENSMUSG00000051285\"; gene_version \"18\"; transcript_id \"ENSMUST00000061280\"; transcript_version \"17\"; exon_number \"5\"; gene_name \"Pcmtd1\"; gene_source \"ensembl_havana\"; gene_biotype \"protein_coding\"; transcript_name \"Pcmtd1-201\"; transcript_source \"ensembl_havana\"; transcript_biotype \"protein_coding\"; tag \"CCDS\"; ccds_id \"CCDS35508\"; exon_id \"ENSMUSE00001298878\"; exon_version \"2\"; tag \"basic\"; tag \"Ensembl_canonical\"; transcript_support_level \"1 (assigned to previous version 16)\";
            1\tensembl_havana\tCDS\t7233472\t7233595\t.\t+\t0\tgene_id \"ENSMUSG00000051285\"; gene_version \"18\"; transcript_id \"ENSMUST00000061280\"; transcript_version \"17\"; exon_number \"5\"; gene_name \"Pcmtd1\"; gene_source \"ensembl_havana\"; gene_biotype \"protein_coding\"; transcript_name \"Pcmtd1-201\"; transcript_source \"ensembl_havana\"; transcript_biotype \"protein_coding\"; tag \"CCDS\"; ccds_id \"CCDS35508\"; protein_id \"ENSMUSP00000059261\"; protein_version \"10\"; tag \"basic\"; tag \"Ensembl_canonical\"; transcript_support_level \"1 (assigned to previous version 16)\";
            1\tensembl_havana\texon\t7239739\t7243852\t.\t+\t.\tgene_id \"ENSMUSG00000051285\"; gene_version \"18\"; transcript_id \"ENSMUST00000061280\"; transcript_version \"17\"; exon_number \"6\"; gene_name \"Pcmtd1\"; gene_source \"ensembl_havana\"; gene_biotype \"protein_coding\"; transcript_name \"Pcmtd1-201\"; transcript_source \"ensembl_havana\"; transcript_biotype \"protein_coding\"; tag \"CCDS\"; ccds_id \"CCDS35508\"; exon_id \"ENSMUSE00000466061\"; exon_version \"7\"; tag \"basic\"; tag \"Ensembl_canonical\"; transcript_support_level \"1 (assigned to previous version 16)\";
            1\tensembl_havana\tCDS\t7239739\t7240103\t.\t+\t2\tgene_id \"ENSMUSG00000051285\"; gene_version \"18\"; transcript_id \"ENSMUST00000061280\"; transcript_version \"17\"; exon_number \"6\"; gene_name \"Pcmtd1\"; gene_source \"ensembl_havana\"; gene_biotype \"protein_coding\"; transcript_name \"Pcmtd1-201\"; transcript_source \"ensembl_havana\"; transcript_biotype \"protein_coding\"; tag \"CCDS\"; ccds_id \"CCDS35508\"; protein_id \"ENSMUSP00000059261\"; protein_version \"10\"; tag \"basic\"; tag \"Ensembl_canonical\"; transcript_support_level \"1 (assigned to previous version 16)\";
            1\tensembl_havana\tstop_codon\t7240104\t7240106\t.\t+\t0\tgene_id \"ENSMUSG00000051285\"; gene_version \"18\"; transcript_id \"ENSMUST00000061280\"; transcript_version \"17\"; exon_number \"6\"; gene_name \"Pcmtd1\"; gene_source \"ensembl_havana\"; gene_biotype \"protein_coding\"; transcript_name \"Pcmtd1-201\"; transcript_source \"ensembl_havana\"; transcript_biotype \"protein_coding\"; tag \"CCDS\"; ccds_id \"CCDS35508\"; tag \"basic\"; tag \"Ensembl_canonical\"; transcript_support_level \"1 (assigned to previous version 16)\";
            1\tensembl_havana\tfive_prime_utr\t7159144\t7159440\t.\t+\t.\tgene_id \"ENSMUSG00000051285\"; gene_version \"18\"; transcript_id \"ENSMUST00000061280\"; transcript_version \"17\"; gene_name \"Pcmtd1\"; gene_source \"ensembl_havana\"; gene_biotype \"protein_coding\"; transcript_name \"Pcmtd1-201\"; transcript_source \"ensembl_havana\"; transcript_biotype \"protein_coding\"; tag \"CCDS\"; ccds_id \"CCDS35508\"; tag \"basic\"; tag \"Ensembl_canonical\"; transcript_support_level \"1 (assigned to previous version 16)\";
            1\tensembl_havana\tthree_prime_utr\t7240107\t7243852\t.\t+\t.\tgene_id \"ENSMUSG00000051285\"; gene_version \"18\"; transcript_id \"ENSMUST00000061280\"; transcript_version \"17\"; gene_name \"Pcmtd1\"; gene_source \"ensembl_havana\"; gene_biotype \"protein_coding\"; transcript_name \"Pcmtd1-201\"; transcript_source \"ensembl_havana\"; transcript_biotype \"protein_coding\"; tag \"CCDS\"; ccds_id \"CCDS35508\"; tag \"basic\"; tag \"Ensembl_canonical\"; transcript_support_level \"1 (assigned to previous version 16)\";"
        );
        
        let mut output_content = String::new();
        let mut output_file = File::open(&output_file).expect("Failed to open output file");

        output_file
        .read_to_string(&mut output_content)
        .expect("Failed to read output file");

        assert_eq!(sorted_content, output_content);

        teardown()
    }


    #[test]
    fn test_gtfsort_outer_order() {
        let input_content = indoc!(
            "chr2\tbed2gtf\tCDS\t164694269\t164695836\t.\t-\t2\tgene_id \"ENSG00000082438\"; transcript_id \"ENST00000375458\"; exon_number \"11\"; exon_id \"ENST00000375458.11\";
            chr2\tbed2gtf\texon\t164692221\t164692397\t.\t-\t.\tgene_id \"ENSG00000082438\"; transcript_id \"ENST00000375458\"; exon_number \"12\"; exon_id \"ENST00000375458.12\";
            chr2\tbed2gtf\tCDS\t164692221\t164692397\t.\t-\t0\tgene_id \"ENSG00000082438\"; transcript_id \"ENST00000375458\"; exon_number \"12\"; exon_id \"ENST00000375458.12\";
            chr2\tbed2gtf\texon\t164680188\t164686032\t.\t-\t.\tgene_id \"ENSG00000082438\"; transcript_id \"ENST00000375458\"; exon_number \"13\"; exon_id \"ENST00000375458.13\";
            chr2\tbed2gtf\tCDS\t164685949\t164686032\t.\t-\t0\tgene_id \"ENSG00000082438\"; transcript_id \"ENST00000375458\"; exon_number \"13\"; exon_id \"ENST00000375458.13\";
            chr2\tbed2gtf\tthree_prime_utr\t164680188\t164685945\t.\t-\t0\tgene_id \"ENSG00000082438\"; transcript_id \"ENST00000375458\";
            chr2\tbed2gtf\ttranscript\t164653624\t164692346\t.\t-\t.\tgene_id \"ENSG00000082438\"; transcript_id \"ENST00000495084\";
            chr1\tbed2gtf\texon\t166847494\t166847592\t.\t+\t.\tgene_id \"ENSG00000143157\"; transcript_id \"ENST00000367875\"; exon_number \"4\"; exon_id \"ENST00000367875.4\";
            chr1\tbed2gtf\tCDS\t166847494\t166847592\t.\t+\t2\tgene_id \"ENSG00000143157\"; transcript_id \"ENST00000367875\"; exon_number \"4\"; exon_id \"ENST00000367875.4\";
            chr1\tbed2gtf\texon\t166848938\t166854473\t.\t+\t.\tgene_id \"ENSG00000143157\"; transcript_id \"ENST00000367875\"; exon_number \"5\"; exon_id \"ENST00000367875.5\";
            chr1\tbed2gtf\tCDS\t166848938\t166850406\t.\t+\t2\tgene_id \"ENSG00000143157\"; transcript_id \"ENST00000367875\"; exon_number \"5\"; exon_id \"ENST00000367875.5\";
            chr1\tbed2gtf\tstop_codon\t166850407\t166850409\t.\t+\t0\tgene_id \"ENSG00000143157\"; transcript_id \"ENST00000367875\"; exon_number \"5\"; exon_id \"ENST00000367875.5\";
            chr1\tbed2gtf\tfive_prime_utr\t166840955\t166840956\t.\t+\t0\tgene_id \"ENSG00000143157\"; transcript_id \"ENST00000367875\";
            chr2\tbed2gtf\tgene\t164653624\t164692346\t.\t-\t.\tgene_id \"ENSG00000082438\";
            chr2\tbed2gtf\ttranscript\t164680188\t164841418\t.\t-\t.\tgene_id \"ENSG00000082438\"; transcript_id \"ENST00000375458\";
            chr2\tbed2gtf\texon\t164700522\t164700756\t.\t-\t.\tgene_id \"ENSG00000082438\"; transcript_id \"ENST00000375458\"; exon_number \"9\"; exon_id \"ENST00000375458.9\";
            chr2\tbed2gtf\texon\t164699405\t164699499\t.\t-\t.\tgene_id \"ENSG00000082438\"; transcript_id \"ENST00000375458\"; exon_number \"10\"; exon_id \"ENST00000375458.10\";
            chr2\tbed2gtf\tCDS\t164699405\t164699499\t.\t-\t1\tgene_id \"ENSG00000082438\"; transcript_id \"ENST00000375458\"; exon_number \"10\"; exon_id \"ENST00000375458.10\";
            chr2\tbed2gtf\texon\t164694269\t164695836\t.\t-\t.\tgene_id \"ENSG00000082438\"; transcript_id \"ENST00000375458\"; exon_number \"11\"; exon_id \"ENST00000375458.11\";
            chr2\tbed2gtf\texon\t164692221\t164692346\t.\t-\t.\tgene_id \"ENSG00000082438\"; transcript_id \"ENST00000495084\"; exon_number \"1\"; exon_id \"ENST00000495084.1\";
            chr2\tbed2gtf\texon\t164665847\t164665901\t.\t-\t.\tgene_id \"ENSG00000082438\"; transcript_id \"ENST00000495084\"; exon_number \"2\"; exon_id \"ENST00000495084.2\";
            chr2\tbed2gtf\texon\t164653624\t164653914\t.\t-\t.\tgene_id \"ENSG00000082438\"; transcript_id \"ENST00000495084\"; exon_number \"3\"; exon_id \"ENST00000495084.3\";
            chr1\tbed2gtf\tthree_prime_utr\t166850410\t166854473\t.\t+\t2\tgene_id \"ENSG00000143157\"; transcript_id \"ENST00000367875\";
            chr1\tbed2gtf\tgene\t166840089\t166854473\t.\t+\t.\tgene_id \"ENSG00000143157\";
            chr1\tbed2gtf\ttranscript\t166840089\t166854473\t.\t+\t.\tgene_id \"ENSG00000143157\"; transcript_id \"ENST00000367875\";
            chr1\tbed2gtf\texon\t166840089\t166840446\t.\t+\t.\tgene_id \"ENSG00000143157\"; transcript_id \"ENST00000367875\"; exon_number \"1\"; exon_id \"ENST00000367875.1\";
            chr1\tbed2gtf\texon\t166840955\t166841088\t.\t+\t.\tgene_id \"ENSG00000143157\"; transcript_id \"ENST00000367875\"; exon_number \"2\"; exon_id \"ENST00000367875.2\";
            chr1\tbed2gtf\tCDS\t166840957\t166841088\t.\t+\t0\tgene_id \"ENSG00000143157\"; transcript_id \"ENST00000367875\"; exon_number \"2\"; exon_id \"ENST00000367875.2\";
            chr1\tbed2gtf\texon\t166846612\t166846738\t.\t+\t.\tgene_id \"ENSG00000143157\"; transcript_id \"ENST00000367875\"; exon_number \"3\"; exon_id \"ENST00000367875.3\";
            chr1\tbed2gtf\tCDS\t166846612\t166846738\t.\t+\t0\tgene_id \"ENSG00000143157\"; transcript_id \"ENST00000367875\"; exon_number \"3\"; exon_id \"ENST00000367875.3\";"
        );
        

        let (input_file, output_file) = create_test_file(input_content);
        let result = gtfsort(&input_file, &output_file);

        println!("{:?}", result);
        assert!(result.is_ok());

        let sorted_content = indoc!(
            "chr1\tbed2gtf\tgene\t166840089\t166854473\t.\t+\t.\tgene_id \"ENSG00000143157\";
            chr1\tbed2gtf\ttranscript\t166840089\t166854473\t.\t+\t.\tgene_id \"ENSG00000143157\"; transcript_id \"ENST00000367875\";
            chr1\tbed2gtf\texon\t166840089\t166840446\t.\t+\t.\tgene_id \"ENSG00000143157\"; transcript_id \"ENST00000367875\"; exon_number \"1\"; exon_id \"ENST00000367875.1\";
            chr1\tbed2gtf\texon\t166840955\t166841088\t.\t+\t.\tgene_id \"ENSG00000143157\"; transcript_id \"ENST00000367875\"; exon_number \"2\"; exon_id \"ENST00000367875.2\";
            chr1\tbed2gtf\tCDS\t166840957\t166841088\t.\t+\t0\tgene_id \"ENSG00000143157\"; transcript_id \"ENST00000367875\"; exon_number \"2\"; exon_id \"ENST00000367875.2\";
            chr1\tbed2gtf\texon\t166846612\t166846738\t.\t+\t.\tgene_id \"ENSG00000143157\"; transcript_id \"ENST00000367875\"; exon_number \"3\"; exon_id \"ENST00000367875.3\";
            chr1\tbed2gtf\tCDS\t166846612\t166846738\t.\t+\t0\tgene_id \"ENSG00000143157\"; transcript_id \"ENST00000367875\"; exon_number \"3\"; exon_id \"ENST00000367875.3\";
            chr1\tbed2gtf\texon\t166847494\t166847592\t.\t+\t.\tgene_id \"ENSG00000143157\"; transcript_id \"ENST00000367875\"; exon_number \"4\"; exon_id \"ENST00000367875.4\";
            chr1\tbed2gtf\tCDS\t166847494\t166847592\t.\t+\t2\tgene_id \"ENSG00000143157\"; transcript_id \"ENST00000367875\"; exon_number \"4\"; exon_id \"ENST00000367875.4\";
            chr1\tbed2gtf\texon\t166848938\t166854473\t.\t+\t.\tgene_id \"ENSG00000143157\"; transcript_id \"ENST00000367875\"; exon_number \"5\"; exon_id \"ENST00000367875.5\";
            chr1\tbed2gtf\tCDS\t166848938\t166850406\t.\t+\t2\tgene_id \"ENSG00000143157\"; transcript_id \"ENST00000367875\"; exon_number \"5\"; exon_id \"ENST00000367875.5\";
            chr1\tbed2gtf\tstop_codon\t166850407\t166850409\t.\t+\t0\tgene_id \"ENSG00000143157\"; transcript_id \"ENST00000367875\"; exon_number \"5\"; exon_id \"ENST00000367875.5\";
            chr1\tbed2gtf\tfive_prime_utr\t166840955\t166840956\t.\t+\t0\tgene_id \"ENSG00000143157\"; transcript_id \"ENST00000367875\";
            chr1\tbed2gtf\tthree_prime_utr\t166850410\t166854473\t.\t+\t2\tgene_id \"ENSG00000143157\"; transcript_id \"ENST00000367875\";
            chr2\tbed2gtf\tgene\t164653624\t164692346\t.\t-\t.\tgene_id \"ENSG00000082438\";
            chr2\tbed2gtf\ttranscript\t164653624\t164692346\t.\t-\t.\tgene_id \"ENSG00000082438\"; transcript_id \"ENST00000495084\";
            chr2\tbed2gtf\texon\t164692221\t164692346\t.\t-\t.\tgene_id \"ENSG00000082438\"; transcript_id \"ENST00000495084\"; exon_number \"1\"; exon_id \"ENST00000495084.1\";
            chr2\tbed2gtf\texon\t164665847\t164665901\t.\t-\t.\tgene_id \"ENSG00000082438\"; transcript_id \"ENST00000495084\"; exon_number \"2\"; exon_id \"ENST00000495084.2\";
            chr2\tbed2gtf\texon\t164653624\t164653914\t.\t-\t.\tgene_id \"ENSG00000082438\"; transcript_id \"ENST00000495084\"; exon_number \"3\"; exon_id \"ENST00000495084.3\";
            chr2\tbed2gtf\ttranscript\t164680188\t164841418\t.\t-\t.\tgene_id \"ENSG00000082438\"; transcript_id \"ENST00000375458\";
            chr2\tbed2gtf\texon\t164700522\t164700756\t.\t-\t.\tgene_id \"ENSG00000082438\"; transcript_id \"ENST00000375458\"; exon_number \"9\"; exon_id \"ENST00000375458.9\";
            chr2\tbed2gtf\texon\t164699405\t164699499\t.\t-\t.\tgene_id \"ENSG00000082438\"; transcript_id \"ENST00000375458\"; exon_number \"10\"; exon_id \"ENST00000375458.10\";
            chr2\tbed2gtf\tCDS\t164699405\t164699499\t.\t-\t1\tgene_id \"ENSG00000082438\"; transcript_id \"ENST00000375458\"; exon_number \"10\"; exon_id \"ENST00000375458.10\";
            chr2\tbed2gtf\texon\t164694269\t164695836\t.\t-\t.\tgene_id \"ENSG00000082438\"; transcript_id \"ENST00000375458\"; exon_number \"11\"; exon_id \"ENST00000375458.11\";
            chr2\tbed2gtf\tCDS\t164694269\t164695836\t.\t-\t2\tgene_id \"ENSG00000082438\"; transcript_id \"ENST00000375458\"; exon_number \"11\"; exon_id \"ENST00000375458.11\";
            chr2\tbed2gtf\texon\t164692221\t164692397\t.\t-\t.\tgene_id \"ENSG00000082438\"; transcript_id \"ENST00000375458\"; exon_number \"12\"; exon_id \"ENST00000375458.12\";
            chr2\tbed2gtf\tCDS\t164692221\t164692397\t.\t-\t0\tgene_id \"ENSG00000082438\"; transcript_id \"ENST00000375458\"; exon_number \"12\"; exon_id \"ENST00000375458.12\";
            chr2\tbed2gtf\texon\t164680188\t164686032\t.\t-\t.\tgene_id \"ENSG00000082438\"; transcript_id \"ENST00000375458\"; exon_number \"13\"; exon_id \"ENST00000375458.13\";
            chr2\tbed2gtf\tCDS\t164685949\t164686032\t.\t-\t0\tgene_id \"ENSG00000082438\"; transcript_id \"ENST00000375458\"; exon_number \"13\"; exon_id \"ENST00000375458.13\";
            chr2\tbed2gtf\tthree_prime_utr\t164680188\t164685945\t.\t-\t0\tgene_id \"ENSG00000082438\"; transcript_id \"ENST00000375458\";"
        );
        
        let mut output_content = String::new();
        let mut output_file = File::open(&output_file).expect("Failed to open output file");

        output_file
        .read_to_string(&mut output_content)
        .expect("Failed to read output file");

        assert_eq!(sorted_content, output_content);

        teardown()
    }


    #[test]
    fn test_gtfsort_empty_input() {

        let input_content = "";
        let (input_file, output_file) = create_test_file(input_content);

        let result = gtfsort(&input_file, &output_file);

        assert!(result.is_err());

        teardown()
    }

    fn teardown() {
        std::fs::remove_file("test_input.gtf").unwrap();
        std::fs::remove_file("test_output.gtf").unwrap();
    }
}
