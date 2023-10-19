#![allow(dead_code)]
use std::collections::HashMap;

use thiserror::Error;

#[derive(Debug, PartialEq)]
pub struct Attribute {
    gene_id: String,
    transcript_id: String,
    exon_number: String,
    exon_id: String,
}

impl Attribute {
    pub fn parse(line: &String) -> Result<Attribute, ParseError> {
        if !line.is_empty() {
            let mut attributes: HashMap<String, String> = HashMap::new();
            let bytes = line.as_bytes().iter().enumerate();

            let mut start = 0;

            for (i, byte) in bytes {
                if *byte == b';' {
                    let word = &line[start..i];
                    if !word.is_empty() {
                        let (key, value) = get_pair(word);
                        attributes.insert(key, value);
                    }
                    start = i + 2;
                }
            }

            let gene_id = attributes.get("gene_id").ok_or(ParseError::Invalid);

            Ok(Attribute {
                gene_id: gene_id?.to_string(),
                transcript_id: attributes
                    .get("transcript_id")
                    .unwrap_or(&"0".to_string())
                    .to_string(),
                exon_number: attributes
                    .get("exon_number")
                    .unwrap_or(&"z".to_string())
                    .to_string(),
                exon_id: attributes
                    .get("exon_id")
                    .unwrap_or(&"0".to_string())
                    .to_string(),
            })
        } else {
            Err(ParseError::Empty)
        }
    }

    pub fn gene_id(&self) -> &str {
        &self.gene_id
    }

    pub fn transcript_id(&self) -> &str {
        &self.transcript_id
    }

    pub fn exon_number(&self) -> &str {
        &self.exon_number
    }

    pub fn exon_id(&self) -> &str {
        &self.exon_id
    }
}

fn get_pair(line: &str) -> (String, String) {
    let mut bytes = line.as_bytes().iter();
    let i = bytes
        .position(|b| *b == b' ')
        .ok_or(ParseError::Invalid)
        .unwrap();
    let key = &line[..i];
    if key == "level" || key == "exon_number" {
        let value = &line[i + 1..line.len()];
        return (key.to_string(), value.to_string());
    } else {
        let value = &line[i + 2..line.len() - 1];
        return (key.to_string(), value.to_string());
    }
}

#[derive(Error, Debug, PartialEq)]
pub enum ParseError {
    #[error("Empty line")]
    Empty,
    #[error("Invalid GTF line")]
    Invalid,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_attributes() {
        let input = "gene_id \"ABC\"; transcript_id \"XYZ\"; exon_number \"1\"; exon_id \"123\";"
            .to_string();
        let attr = Attribute::parse(&input).unwrap();

        assert_eq!(attr.gene_id(), "ABC");
        assert_eq!(attr.transcript_id(), "XYZ");
        assert_eq!(attr.exon_number(), "1");
        assert_eq!(attr.exon_id(), "123");
    }

    #[test]
    fn invalid_attributes() {
        let input = "transcript_id \"XYZ\"; exon_number \"1\";".to_string();
        let result = Attribute::parse(&input);

        assert_eq!(result.unwrap_err(), ParseError::Invalid);
    }

    #[test]
    fn get_gencode_pair_from_gene_line() {
        let line = "gene_id \"ENSG00000290825.1\"; gene_type \"lncRNA\"; gene_name \"DDX11L2\"; level 2; tag \"overlaps_pseudogene\";".to_string();
        let mut attributes: HashMap<String, String> = HashMap::new();
        let bytes = line.as_bytes().iter().enumerate();

        let mut start = 0;

        for (i, byte) in bytes {
            if *byte == b';' {
                let word = &line[start..i];
                if !word.is_empty() {
                    let (key, value) = get_pair(word);
                    attributes.insert(key, value);
                }
                start = i + 2;
            }
        }

        assert_eq!(
            *attributes.get("gene_id").unwrap(),
            String::from("ENSG00000290825.1")
        );
        assert_eq!(
            *attributes.get("gene_type").unwrap(),
            String::from("lncRNA")
        );
        assert_eq!(
            *attributes.get("gene_name").unwrap(),
            String::from("DDX11L2")
        );
        assert_eq!(*attributes.get("level").unwrap(), String::from("2"));
        assert_eq!(
            *attributes.get("tag").unwrap(),
            String::from("overlaps_pseudogene")
        );
    }
}

#[test]
fn get_gencode_pair_from_exon_line() {
    let line = "gene_id \"ENSG00000290825.1\"; transcript_id \"ENST00000456328.2\"; gene_type \"lncRNA\"; gene_name \"DDX11L2\"; transcript_type \"lncRNA\"; transcript_name \"DDX11L2-202\"; exon_number 2; exon_id \"ENSE00003582793.1\"; level 2; transcript_support_level \"1\"; tag \"basic\"; tag \"Ensembl_canonical\"; havana_transcript \"OTTHUMT00000362751.1\";".to_string();
    let mut attributes: HashMap<String, String> = HashMap::new();
    let bytes = line.as_bytes().iter().enumerate();

    let mut start = 0;

    for (i, byte) in bytes {
        if *byte == b';' {
            let word = &line[start..i];
            if !word.is_empty() {
                let (key, value) = get_pair(word);
                attributes.insert(key, value);
            }
            start = i + 2;
        }
    }

    assert_eq!(
        *attributes.get("gene_id").unwrap(),
        String::from("ENSG00000290825.1")
    );
    assert_eq!(
        *attributes.get("transcript_id").unwrap(),
        String::from("ENST00000456328.2")
    );
    assert_eq!(
        *attributes.get("gene_type").unwrap(),
        String::from("lncRNA")
    );
    assert_eq!(
        *attributes.get("gene_name").unwrap(),
        String::from("DDX11L2")
    );
    assert_eq!(
        *attributes.get("transcript_type").unwrap(),
        String::from("lncRNA")
    );
    assert_eq!(
        *attributes.get("transcript_name").unwrap(),
        String::from("DDX11L2-202")
    );
    assert_eq!(*attributes.get("exon_number").unwrap(), String::from("2"));
    assert_eq!(
        *attributes.get("exon_id").unwrap(),
        String::from("ENSE00003582793.1")
    );
    assert_eq!(*attributes.get("level").unwrap(), String::from("2"));
    assert_eq!(
        *attributes.get("transcript_support_level").unwrap(),
        String::from("1")
    );
    assert_eq!(
        *attributes.get("tag").unwrap(),
        String::from("Ensembl_canonical")
    );
    assert_eq!(
        *attributes.get("havana_transcript").unwrap(),
        String::from("OTTHUMT00000362751.1")
    );
}
