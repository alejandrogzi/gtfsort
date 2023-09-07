use std::collections::HashMap;

use thiserror::Error;


#[derive(Debug)]
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
            Ok(Attribute {
                gene_id: attributes.get("gene_id").ok_or(ParseError::Invalid).unwrap().to_string(),
                transcript_id: attributes.get("transcript_id").unwrap_or(&"0".to_string()).to_string(),
                exon_number: attributes.get("exon_number").unwrap_or(&"z".to_string()).to_string(),
                exon_id: attributes.get("exon_id").unwrap_or(&"0".to_string()).to_string(),
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
    let i = bytes.position(|b| *b == b' ').ok_or(ParseError::Invalid).unwrap();
    let key = &line[..i];
    let value = &line[i+2..line.len()-1];

    (key.to_string(), value.to_string())
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
    fn test_parse_valid_input() {
        let input = "gene_id \"ABC\"; transcript_id \"XYZ\"; exon_number \"1\"; exon_id \"123\"".to_string();
        let attr = Attribute::parse(&input).unwrap();

        assert_eq!(attr.gene_id(), "ABC");
        assert_eq!(attr.transcript_id(), "XYZ");
        assert_eq!(attr.exon_number(), "1");
        assert_eq!(attr.exon_id(), "123");
    }

    #[test]
    fn test_parse_invalid_input() {
        let input = "gene_id \"ABC\"; transcript_id \"XYZ\"; exon_number \"1\"".to_string();
        let result = Attribute::parse(&input).unwrap();

        assert_eq!(result.gene_id(), "ABC");
        assert_eq!(result.transcript_id(), "XYZ");
        assert_eq!(result.exon_number(), "1");
        assert_eq!(result.exon_id(), "Invalid GTF line");
    }
}