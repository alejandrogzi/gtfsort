use std::error::Error;

#[derive(Debug, PartialEq, Eq, Ord, PartialOrd)]
pub struct Record {
    pub chrom: String,
    pub feat: String,
    pub pos: i32,
    pub gene_id: String,
    pub transcript_id: String,
    pub exon_number: String,
    pub line: String,
}

impl Record {
    pub fn new(line: &str) -> Self {
        let fields: Vec<&str> = line.trim().split('\t').collect();
        
        let mut gp = Record {
            chrom: fields[0].to_string(),
            feat: fields[2].to_string(),
            pos: fields[3].parse().unwrap(),
            gene_id: String::new(),
            transcript_id: String::new(),
            exon_number: String::new(),
            line: line.to_string(),
        };

        match gp.feat.as_str() {
            "gene" => {
                gp.gene_id = Record::get_attribute("gene_id", &fields).unwrap();
            }
            "exon" | "CDS" | "start_codon" | "stop_codon" => {
                gp.gene_id = Record::get_attribute("gene_id", &fields).unwrap();
                gp.transcript_id = Record::get_attribute("transcript_id", &fields).unwrap();
                gp.exon_number = Record::get_attribute("exon_number", &fields).unwrap();
            }
            _ => {
                gp.gene_id = Record::get_attribute("gene_id", &fields).unwrap();
                gp.transcript_id = Record::get_attribute("transcript_id", &fields).unwrap();
            }
        }
        
        gp
    }

    fn get_attribute(attr: &str, fields: &Vec<&str>) -> Result<String, Box<dyn Error>> {
        let mut attributes = fields.last().unwrap().split("; ");
        let attribute = attributes
                                    .find(|x| x.starts_with(attr))
                                    .unwrap()
                                    .split(" ")
                                    .nth(1)
                                    .unwrap()
                                    .replace('"', "")
                                    .to_string();
        Ok(attribute)
    }
}