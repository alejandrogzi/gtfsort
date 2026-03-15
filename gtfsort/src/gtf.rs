mod attr;
use std::borrow::Cow;

pub use attr::*;

#[derive(Debug, PartialEq, Eq, Ord, PartialOrd, Clone, Copy)]
pub struct Record<'a> {
    pub line_no: usize,
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
    /// Parses a single tab-delimited GTF/GFF line into a borrowed record view.
    #[inline]
    pub fn parse<const SEP: u8>(line_no: usize, line: &'a str) -> Result<Self, Cow<'static, str>> {
        if line.is_empty() {
            return Err("Empty line".into());
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

        let attributes = Attribute::parse::<SEP>(attrs_str).map_err(|e| e.to_string())?;

        Ok(Self {
            line_no,
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

    /// Returns the outer sorting key for gene feature lines.
    #[inline(always)]
    pub fn outer_layer(&self) -> (u32, &'a str, &'a str) {
        (self.start, self.gene_id, self.line)
    }

    /// Returns the legacy exon/codon ordering key for transcript child features.
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

    /// Returns true when the record represents a gene feature row.
    #[inline(always)]
    pub fn is_gene(&self) -> bool {
        self.feat == "gene"
    }

    /// Returns true when the record represents a transcript feature row.
    #[inline(always)]
    pub fn is_transcript(&self) -> bool {
        self.feat == "transcript"
    }

    /// Returns true when the record belongs to a transcript block.
    #[inline(always)]
    pub fn has_transcript(&self) -> bool {
        self.transcript_id != "0"
    }
}

mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn valid_record() {
        let line = "1\thavana\tCDS\t2408530\t2408619\t.\t-\t0\tgene_id \"ENSG00000157911\"; gene_version \"11\"; transcript_id \"ENST00000508384\"; transcript_version \"5\"; exon_number \"3\"; gene_name \"PEX10\"; gene_source \"ensembl_havana\"; gene_biotype \"protein_coding\"; transcript_name \"PEX10-205\"; transcript_source \"havana\"; transcript_biotype \"protein_coding\"; protein_id \"ENSP00000464289\"; protein_version \"1\"; tag \"cds_end_NF\"; tag \"mRNA_end_NF\"; transcript_support_level \"3\";".to_string();
        let result = Record::parse::<b' '>(0, &line);

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
        let result = Record::parse::<b' '>(0, &line);

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Empty line");
    }

    #[test]
    fn outer_layer() {
        let line = "1\thavana\tCDS\t2408530\t2408619\t.\t-\t0\tgene_id \"ENSG00000157911\"; gene_version \"11\"; transcript_id \"ENST00000508384\"; transcript_version \"5\"; exon_number \"3\"; gene_name \"PEX10\"; gene_source \"ensembl_havana\"; gene_biotype \"protein_coding\"; transcript_name \"PEX10-205\"; transcript_source \"havana\"; transcript_biotype \"protein_coding\"; protein_id \"ENSP00000464289\"; protein_version \"1\"; tag \"cds_end_NF\"; tag \"mRNA_end_NF\"; transcript_support_level \"3\";".to_string();
        let record = Record::parse::<b' '>(0, &line).unwrap();
        let (start, gene_id, line) = record.outer_layer();

        assert_eq!(start, 2408530);
        assert_eq!(gene_id, "ENSG00000157911");
        assert_eq!(line, "1\thavana\tCDS\t2408530\t2408619\t.\t-\t0\tgene_id \"ENSG00000157911\"; gene_version \"11\"; transcript_id \"ENST00000508384\"; transcript_version \"5\"; exon_number \"3\"; gene_name \"PEX10\"; gene_source \"ensembl_havana\"; gene_biotype \"protein_coding\"; transcript_name \"PEX10-205\"; transcript_source \"havana\"; transcript_biotype \"protein_coding\"; protein_id \"ENSP00000464289\"; protein_version \"1\"; tag \"cds_end_NF\"; tag \"mRNA_end_NF\"; transcript_support_level \"3\";");
    }
}
