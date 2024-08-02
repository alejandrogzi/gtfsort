mod attr;
pub use attr::*;

#[derive(Debug, PartialEq, Eq, Ord, PartialOrd)]
pub struct Record<'a> {
    pub chrom: &'a str,
    pub feat: &'a str,
    pub start: u32,
    pub end: u32,
    pub gene_id: &'a str,
    pub transcript_id: &'a str,
    pub exon_number: &'a str,
    pub line: &'a str,
}

impl<'a> Record<'a> {
    #[inline]
    pub fn parse<const SEP: u8>(line: &'a str) -> Result<Self, &'static str> {
        if line.is_empty() {
            return Err("Empty line");
        }

        let mut fields = line.split('\t');
        let (chrom, _, feat, start, end, _, _, _, attrs_str) = (
            fields.next().ok_or("Missing chrom")?,
            fields.next().ok_or("Missing source")?,
            fields.next().ok_or("Missing feature")?,
            fields.next().ok_or("Missing start")?,
            fields.next().ok_or("Missing end")?,
            fields.next().ok_or("Missing score")?,
            fields.next().ok_or("Missing strand")?,
            fields.next().ok_or("Missing frame")?,
            fields.next().ok_or("Missing attributes")?,
        );

        let attributes = Attribute::parse::<SEP>(attrs_str).unwrap();

        Ok(Self {
            chrom,
            feat,
            start: start.parse().map_err(|_| "Invalid start")?,
            end: end.parse().map_err(|_| "Invalid end")?,
            gene_id: attributes.gene_id(),
            transcript_id: attributes.transcript_id(),
            exon_number: attributes.exon_number(),
            line,
        })
    }

    #[inline(always)]
    pub fn outer_layer(&self) -> (u32, &'a str, &'a str) {
        (self.start, self.gene_id, self.line)
    }

    #[inline(always)]
    pub fn inner_layer(&self) -> (&'a str, char) {
        (
            self.exon_number,
            match self.feat {
                "exon" => 'a',
                "CDS" => 'b',
                "start_codon" => 'c',
                "stop_codon" => 'd',
                _ => 'e',
            },
        )
    }
}

mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn valid_record() {
        let line = "1\thavana\tCDS\t2408530\t2408619\t.\t-\t0\tgene_id \"ENSG00000157911\"; gene_version \"11\"; transcript_id \"ENST00000508384\"; transcript_version \"5\"; exon_number \"3\"; gene_name \"PEX10\"; gene_source \"ensembl_havana\"; gene_biotype \"protein_coding\"; transcript_name \"PEX10-205\"; transcript_source \"havana\"; transcript_biotype \"protein_coding\"; protein_id \"ENSP00000464289\"; protein_version \"1\"; tag \"cds_end_NF\"; tag \"mRNA_end_NF\"; transcript_support_level \"3\";".to_string();
        let result = Record::parse::<b' '>(&line);

        assert!(result.is_ok());

        let record = result.unwrap();
        assert_eq!(record.chrom, "1");
        assert_eq!(record.feat, "CDS");
        assert_eq!(record.start, 2408530);
        assert_eq!(record.gene_id, "ENSG00000157911");
        assert_eq!(record.transcript_id, "ENST00000508384");
        assert_eq!(record.exon_number, "3");
        assert_eq!(record.line, line);
    }

    #[test]
    fn empty_record() {
        let line = "".to_string();
        let result = Record::parse::<b' '>(&line);

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Empty line");
    }

    #[test]
    fn outer_layer() {
        let line = "1\thavana\tCDS\t2408530\t2408619\t.\t-\t0\tgene_id \"ENSG00000157911\"; gene_version \"11\"; transcript_id \"ENST00000508384\"; transcript_version \"5\"; exon_number \"3\"; gene_name \"PEX10\"; gene_source \"ensembl_havana\"; gene_biotype \"protein_coding\"; transcript_name \"PEX10-205\"; transcript_source \"havana\"; transcript_biotype \"protein_coding\"; protein_id \"ENSP00000464289\"; protein_version \"1\"; tag \"cds_end_NF\"; tag \"mRNA_end_NF\"; transcript_support_level \"3\";".to_string();
        let record = Record::parse::<b' '>(&line).unwrap();
        let (start, gene_id, line) = record.outer_layer();

        assert_eq!(start, 2408530);
        assert_eq!(gene_id, "ENSG00000157911");
        assert_eq!(line, "1\thavana\tCDS\t2408530\t2408619\t.\t-\t0\tgene_id \"ENSG00000157911\"; gene_version \"11\"; transcript_id \"ENST00000508384\"; transcript_version \"5\"; exon_number \"3\"; gene_name \"PEX10\"; gene_source \"ensembl_havana\"; gene_biotype \"protein_coding\"; transcript_name \"PEX10-205\"; transcript_source \"havana\"; transcript_biotype \"protein_coding\"; protein_id \"ENSP00000464289\"; protein_version \"1\"; tag \"cds_end_NF\"; tag \"mRNA_end_NF\"; transcript_support_level \"3\";");
    }
}
