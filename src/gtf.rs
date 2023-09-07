mod attr;
pub use attr::*;


#[derive(Debug, PartialEq, Eq, Ord, PartialOrd)]
pub struct Record {
    chrom: String,
    feat: String,
    pos: i32,
    gene_id: String,
    transcript_id: String,
    exon_number: String,
    line: String,
}

impl Record {
    pub fn new(line: String) -> Result<Self, ParseError> {
        if line.is_empty() {
            return Err(ParseError::Empty);
        }

        let fields = splitb(line)?;
        let attributes = Attribute::parse(&fields[8])?;

        Ok(Record {
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
        (self.chrom.clone(), self.pos, self.gene_id.clone(), self.line.clone())
    }

    pub fn gene_to_transcript(&self) -> (String, String, String) {
        (self.gene_id.clone(), self.transcript_id.clone(), self.line.clone())
    }

    pub fn inner_layer(&self) -> (String, String, String) {
        let mut exon_number = self.exon_number.clone();
        match self.feat.as_str() {
            "exon" => exon_number.push('a'),
            "CDS" => exon_number.push('b'),
            "start_codon" | "stop_codon" => exon_number.push('c'),
            "five_prime_utr" => exon_number.push('d'),
            "three_prime_utr" => exon_number.push('e'),
            "UTR" => exon_number.push('f'),
            _ => exon_number.push('e'),
        }
        (self.transcript_id.clone(), exon_number, self.line.clone())
    }

    pub fn feature(&self) -> &str {
        &self.feat
    }

}


fn splitb(line: String) -> Result<Vec<String>, ParseError> {
    let bytes = line.as_bytes().iter().enumerate();
    let mut start = 0;
    let mut entries = Vec::new();

    for (i, byte) in bytes {
        if *byte == b'\t' {
            let word = line[start..i].to_string();
            if !word.is_empty() {
                entries.push(word);
            }
            start = i + 1;
        }
    }
    entries.push(line[start..].to_string());
    Ok(entries)
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_new_valid_input() {
        let line = "chr1\tfeature\t123\tgene123\ttranscript123\texon123\tattribute_data".to_string();
        let result = Record::new(line.clone());

        assert!(result.is_ok());

        let record = result.unwrap();
        assert_eq!(record.chrom, "chr1");
        assert_eq!(record.feat, "feature");
        assert_eq!(record.pos, 123);
        assert_eq!(record.gene_id, "gene123");
        assert_eq!(record.transcript_id, "transcript123");
        assert_eq!(record.exon_number, "exon123");
        assert_eq!(record.line, line);
    }

    #[test]
    fn test_record_new_empty_input() {
        let line = "".to_string();
        let result = Record::new(line);

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), ParseError::Empty);
    }

    #[test]
    fn test_record_new_invalid_input() {
        // Missing fields, which should trigger a ParseError::Invalid
        let line = "chr1\tfeature".to_string();
        let result = Record::new(line);

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), ParseError::Invalid);
    }

    #[test]
    fn test_outer_layer() {
        let line = "chr1\tfeature\t123\tgene123\ttranscript123\texon123\tattribute_data".to_string();
        let record = Record::new(line).unwrap();
        let (chrom, pos, gene_id, line) = record.outer_layer();

        assert_eq!(chrom, "chr1");
        assert_eq!(pos, 123);
        assert_eq!(gene_id, "gene123");
        assert_eq!(line, "chr1\tfeature\t123\tgene123\ttranscript123\texon123\tattribute_data");
    }
}
