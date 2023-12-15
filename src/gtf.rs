mod attr;
pub use attr::*;

#[derive(Debug, PartialEq, Eq, Ord, PartialOrd)]
pub struct Record {
    pub chrom: String,
    feat: String,
    pos: i32,
    gene_id: String,
    transcript_id: String,
    exon_number: String,
    pub line: String,
}

impl Record {
    pub fn parse(line: &str) -> Result<Self, &'static str> {
        if line.is_empty() {
            return Err("Empty line");
        }

        if line.starts_with("#") {
            return Ok(Self {
                chrom: "".to_string(),
                feat: "".to_string(),
                pos: 0,
                gene_id: "".to_string(),
                transcript_id: "".to_string(),
                exon_number: "".to_string(),
                line: line.to_string(),
            });
        }

        let fields: Vec<&str> = line.split("\t").collect();
        let attributes = Attribute::parse(&fields[8].to_string()).unwrap();

        Ok(Self {
            chrom: fields[0].to_string(),
            feat: fields[2].to_string(),
            pos: fields[3].parse().unwrap(),
            gene_id: attributes.gene_id().to_string(),
            transcript_id: attributes.transcript_id().to_string(),
            exon_number: attributes.exon_number().to_string(),
            line: fields.join("\t"),
        })
    }

    pub fn outer_layer(&self) -> (String, i32, String, String) {
        (
            self.chrom.clone(),
            self.pos,
            self.gene_id.clone(),
            self.line.clone(),
        )
    }

    pub fn gene_to_transcript(&self) -> (String, String, String) {
        (
            self.gene_id.clone(),
            self.transcript_id.clone(),
            self.line.clone(),
        )
    }

    pub fn inner_layer(&self) -> (String, String, String) {
        let mut exon_number = self.exon_number.clone();
        match self.feat.as_str() {
            "exon" => exon_number.push('a'),
            "CDS" => exon_number.push('b'),
            "start_codon" => exon_number.push('c'),
            "stop_codon" => exon_number.push('d'),
            _ => exon_number.push('e'),
        }
        (self.transcript_id.clone(), exon_number, self.line.clone())
    }

    pub fn misc_layer(&self) -> (String, String, String) {
        (
            self.transcript_id.clone(),
            self.feat.clone(),
            self.line.clone(),
        )
    }

    pub fn feature(&self) -> &str {
        &self.feat
    }
}

mod tests {
    use super::*;

    #[test]
    fn valid_record() {
        let line = "1\thavana\tCDS\t2408530\t2408619\t.\t-\t0\tgene_id \"ENSG00000157911\"; gene_version \"11\"; transcript_id \"ENST00000508384\"; transcript_version \"5\"; exon_number \"3\"; gene_name \"PEX10\"; gene_source \"ensembl_havana\"; gene_biotype \"protein_coding\"; transcript_name \"PEX10-205\"; transcript_source \"havana\"; transcript_biotype \"protein_coding\"; protein_id \"ENSP00000464289\"; protein_version \"1\"; tag \"cds_end_NF\"; tag \"mRNA_end_NF\"; transcript_support_level \"3\";".to_string();
        let result = Record::parse(&line.clone());

        assert!(result.is_ok());

        let record = result.unwrap();
        assert_eq!(record.chrom, "1");
        assert_eq!(record.feat, "CDS");
        assert_eq!(record.pos, 2408530);
        assert_eq!(record.gene_id, "ENSG00000157911");
        assert_eq!(record.transcript_id, "ENST00000508384");
        assert_eq!(record.exon_number, "3");
        assert_eq!(record.line, line);
    }

    #[test]
    fn empty_record() {
        let line = "".to_string();
        let result = Record::parse(&line);

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Empty line");
    }

    #[test]
    fn outer_layer() {
        let line = "1\thavana\tCDS\t2408530\t2408619\t.\t-\t0\tgene_id \"ENSG00000157911\"; gene_version \"11\"; transcript_id \"ENST00000508384\"; transcript_version \"5\"; exon_number \"3\"; gene_name \"PEX10\"; gene_source \"ensembl_havana\"; gene_biotype \"protein_coding\"; transcript_name \"PEX10-205\"; transcript_source \"havana\"; transcript_biotype \"protein_coding\"; protein_id \"ENSP00000464289\"; protein_version \"1\"; tag \"cds_end_NF\"; tag \"mRNA_end_NF\"; transcript_support_level \"3\";".to_string();
        let record = Record::parse(&line).unwrap();
        let (chrom, pos, gene_id, line) = record.outer_layer();

        assert_eq!(chrom, "1");
        assert_eq!(pos, 2408530);
        assert_eq!(gene_id, "ENSG00000157911");
        assert_eq!(line, "1\thavana\tCDS\t2408530\t2408619\t.\t-\t0\tgene_id \"ENSG00000157911\"; gene_version \"11\"; transcript_id \"ENST00000508384\"; transcript_version \"5\"; exon_number \"3\"; gene_name \"PEX10\"; gene_source \"ensembl_havana\"; gene_biotype \"protein_coding\"; transcript_name \"PEX10-205\"; transcript_source \"havana\"; transcript_biotype \"protein_coding\"; protein_id \"ENSP00000464289\"; protein_version \"1\"; tag \"cds_end_NF\"; tag \"mRNA_end_NF\"; transcript_support_level \"3\";");
    }
}
