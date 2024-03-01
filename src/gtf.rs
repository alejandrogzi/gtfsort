mod attr;
pub use attr::*;

use std::sync::Arc;

#[derive(Debug, PartialEq, Eq, Ord, PartialOrd)]
pub struct Record {
    pub chrom: Arc<str>,
    pub feat: String,
    pub start: u32,
    pub end: u32,
    pub gene_id: Arc<str>,
    pub transcript_id: Arc<str>,
    pub exon_number: String,
    pub line: Arc<str>,
}

impl Record {
    pub fn parse(line: &str) -> Result<Self, &'static str> {
        if line.is_empty() {
            return Err("Empty line");
        }

        let fields: Vec<&str> = line.split("\t").collect();
        let attributes = Attribute::parse(&fields[8].to_string()).unwrap();

        Ok(Self {
            chrom: Arc::from(fields[0]),
            feat: fields[2].to_string(),
            start: fields[3].parse().unwrap(),
            end: fields[4].parse().unwrap(),
            gene_id: Arc::from(attributes.gene_id()),
            transcript_id: attributes.transcript_id().into(),
            exon_number: attributes.exon_number().to_string(),
            line: fields.join("\t").into(),
        })
    }
    pub fn outer_layer(&self) -> (u32, Arc<str>, Arc<str>) {
        (self.start, self.gene_id.clone(), self.line.clone())
    }

    pub fn inner_layer(&self) -> String {
        let mut exon_number = self.exon_number.clone();
        match self.feat.as_str() {
            "exon" => exon_number.push('a'),
            "CDS" => exon_number.push('b'),
            "start_codon" => exon_number.push('c'),
            "stop_codon" => exon_number.push('d'),
            _ => exon_number.push('e'),
        }
        exon_number
    }
}

mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn valid_record() {
        let line = "1\thavana\tCDS\t2408530\t2408619\t.\t-\t0\tgene_id \"ENSG00000157911\"; gene_version \"11\"; transcript_id \"ENST00000508384\"; transcript_version \"5\"; exon_number \"3\"; gene_name \"PEX10\"; gene_source \"ensembl_havana\"; gene_biotype \"protein_coding\"; transcript_name \"PEX10-205\"; transcript_source \"havana\"; transcript_biotype \"protein_coding\"; protein_id \"ENSP00000464289\"; protein_version \"1\"; tag \"cds_end_NF\"; tag \"mRNA_end_NF\"; transcript_support_level \"3\";".to_string();
        let result = Record::parse(&line.clone());

        assert!(result.is_ok());

        let record = result.unwrap();
        assert_eq!(record.chrom, "1".into());
        assert_eq!(record.feat, "CDS");
        assert_eq!(record.start, 2408530);
        assert_eq!(record.gene_id, "ENSG00000157911".into());
        assert_eq!(record.transcript_id, "ENST00000508384".into());
        assert_eq!(record.exon_number, "3");
        assert_eq!(record.line, line.into());
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
        let (start, gene_id, line) = record.outer_layer();

        assert_eq!(start, 2408530);
        assert_eq!(gene_id, "ENSG00000157911".into());
        assert_eq!(line, "1\thavana\tCDS\t2408530\t2408619\t.\t-\t0\tgene_id \"ENSG00000157911\"; gene_version \"11\"; transcript_id \"ENST00000508384\"; transcript_version \"5\"; exon_number \"3\"; gene_name \"PEX10\"; gene_source \"ensembl_havana\"; gene_biotype \"protein_coding\"; transcript_name \"PEX10-205\"; transcript_source \"havana\"; transcript_biotype \"protein_coding\"; protein_id \"ENSP00000464289\"; protein_version \"1\"; tag \"cds_end_NF\"; tag \"mRNA_end_NF\"; transcript_support_level \"3\";".into());
    }
}
