#![allow(dead_code)]

use thiserror::Error;

macro_rules! extract_field {
    ($bytes:ident split by $sep:ident to $( $field_name:literal => $output_field:expr; )+) => {
        $(
            if let Some(without_key) = $bytes.strip_prefix($field_name) {
                if let Some(without_eq) = without_key.strip_prefix(&[$sep]) {
                    let value = unsafe { std::str::from_utf8_unchecked(without_eq) };
                    *$output_field = Some(value.trim_matches(|c| c == '"'));
                }
            }
        )+
    };
    ($bytes:ident split by $sep:literal to $( $field_name:literal => $output_field:expr; )+) => {
        $(
            if let Some(without_key) = $bytes.strip_prefix($field_name) {
                if let Some(without_eq) = without_key.strip_prefix(&[$sep]) {
                    let value = unsafe { std::str::from_utf8_unchecked(without_eq) };
                    *$output_field = Some(value.trim_matches(|c| c == '"'));
                }
            }
        )+
    };
}

#[inline(always)]
fn split_and_trim_bytes<const BY: u8, const TRIM: u8>(bytes: &[u8]) -> impl Iterator<Item = &[u8]> {
    bytes.split(|b| *b == BY).map(|b| {
        let mut idx = 0;
        while idx < b.len() && b[idx] == TRIM {
            idx += 1;
        }
        &b[idx..]
    })
}

#[derive(Debug, PartialEq)]
pub struct Attribute<'a> {
    gene_id: &'a str,
    transcript_id: &'a str,
    exon_number: &'a str,
    exon_id: &'a str,
}

impl<'a> Attribute<'a> {
    pub fn parse<const SEP: u8>(line: &'a str) -> Result<Attribute<'a>, ParseError> {
        if !line.is_empty() {
            let field_bytes = split_and_trim_bytes::<b';', b' '>(line.trim_end().as_bytes());

            let (mut gene_id, mut transcript_id, mut exon_number, mut exon_id) =
                (None, None, None, None);

            for field in field_bytes {
                extract_field!(
                    field split by SEP to
                    b"gene_id" => (&mut gene_id);
                    b"transcript_id" => (&mut transcript_id);
                    b"exon_number" => (&mut exon_number);
                    b"exon_id" => (&mut exon_id););
            }

            Ok(Attribute {
                gene_id: gene_id.ok_or(ParseError::MissingGeneId(line.to_string()))?,
                transcript_id: transcript_id.unwrap_or("0"),
                exon_number: exon_number.unwrap_or("z"),
                exon_id: exon_id.unwrap_or("0"),
            })
        } else {
            Err(ParseError::Empty)
        }
    }

    #[inline(always)]
    pub fn gene_id(&self) -> &'a str {
        self.gene_id
    }

    #[inline(always)]
    pub fn transcript_id(&self) -> &'a str {
        self.transcript_id
    }

    #[inline(always)]
    pub fn exon_number(&self) -> &'a str {
        self.exon_number
    }

    #[inline(always)]
    pub fn exon_id(&self) -> &'a str {
        self.exon_id
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ParseError {
    // Empty line
    #[error("Empty line, cannot parse attributes")]
    Empty,

    // Invalid GTF line (unused for now)
    #[error("Invalid GTF line: {0}")]
    Invalid(String),

    // Invalid attribute pair, allow get_pair panic
    #[error("Invalid attribute pair: {0}")]
    InvalidPair(String),

    // Missing gene_id attribute
    #[error("Missing gene_id attribute in: {0}")]
    MissingGeneId(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_attributes() {
        let input = "gene_id \"ABC\"; transcript_id \"XYZ\"; exon_number \"1\"; exon_id \"123\";"
            .to_string();
        let attr = Attribute::parse::<b' '>(&input).unwrap();

        assert_eq!(attr.gene_id(), "ABC");
        assert_eq!(attr.transcript_id(), "XYZ");
        assert_eq!(attr.exon_number(), "1");
        assert_eq!(attr.exon_id(), "123");
    }

    #[test]
    fn invalid_attributes() {
        let input = "transcript_id \"XYZ\"; exon_number \"1\";".to_string();
        let result = Attribute::parse::<b' '>(&input);

        assert_eq!(result.unwrap_err(), ParseError::MissingGeneId(input));
    }

    #[test]
    fn get_gencode_pair_from_gene_line() {
        let line = "gene_id \"ENSG00000290825.1\"; gene_type \"lncRNA\"; gene_name \"DDX11L2\"; level 2; tag \"overlaps_pseudogene\";".to_string();

        let attrs = Attribute::parse::<b' '>(&line).unwrap();

        assert_eq!(attrs.gene_id(), String::from("ENSG00000290825.1"));

        let (mut gene_id, mut gene_type, mut gene_name, mut level, mut tag) =
            (None, None, None, None, None);

        let bytes = split_and_trim_bytes::<b';', b' '>(line.trim_end().as_bytes());
        for field in bytes {
            extract_field!(
                field split by b' ' to
                b"gene_id" => (&mut gene_id);
                b"gene_type" => (&mut gene_type);
                b"gene_name" => (&mut gene_name);
                b"level" => (&mut level);
                b"tag" => (&mut tag);
            );
        }

        assert_eq!(gene_type, Some("lncRNA"));
        assert_eq!(gene_name, Some("DDX11L2"));
        assert_eq!(level, Some("2"));
        assert_eq!(tag, Some("overlaps_pseudogene"));
    }

    #[test]
    fn get_gencode_pair_from_exon_line() {
        let line = "gene_id \"ENSG00000290825.1\"; transcript_id \"ENST00000456328.2\"; gene_type \"lncRNA\"; gene_name \"DDX11L2\"; transcript_type \"lncRNA\"; transcript_name \"DDX11L2-202\"; exon_number 2; exon_id \"ENSE00003582793.1\"; level 2; transcript_support_level \"1\"; tag \"basic\"; tag \"Ensembl_canonical\"; havana_transcript \"OTTHUMT00000362751.1\";".to_string();

        let (
            mut gene_id,
            mut transcript_id,
            mut gene_type,
            mut gene_name,
            mut transcript_type,
            mut transcript_name,
            mut exon_number,
            mut exon_id,
            mut level,
            mut transcript_support_level,
            mut tag,
            mut havana_transcript,
        ) = (
            None, None, None, None, None, None, None, None, None, None, None, None,
        );

        let bytes = split_and_trim_bytes::<b';', b' '>(line.trim_end().as_bytes());
        for field in bytes {
            extract_field!(
            field split by b' ' to
            b"gene_id" => (&mut gene_id);
            b"transcript_id" => (&mut transcript_id);
            b"gene_type" => (&mut gene_type);
            b"gene_name" => (&mut gene_name);
            b"transcript_type" => (&mut transcript_type);
            b"transcript_name" => (&mut transcript_name);
            b"exon_number" => (&mut exon_number);
            b"exon_id" => (&mut exon_id);
            b"level" => (&mut level);
            b"transcript_support_level" => (&mut transcript_support_level);
            b"tag" => (&mut tag);
            b"havana_transcript" => (&mut havana_transcript);
            );
        }

        assert_eq!(gene_id.unwrap(), String::from("ENSG00000290825.1"));
        assert_eq!(transcript_id.unwrap(), String::from("ENST00000456328.2"));
        assert_eq!(gene_type.unwrap(), String::from("lncRNA"));
        assert_eq!(gene_name.unwrap(), String::from("DDX11L2"));
        assert_eq!(transcript_type.unwrap(), String::from("lncRNA"));
        assert_eq!(transcript_name.unwrap(), String::from("DDX11L2-202"));
        assert_eq!(exon_number.unwrap(), String::from("2"));
        assert_eq!(exon_id.unwrap(), String::from("ENSE00003582793.1"));
        assert_eq!(level.unwrap(), String::from("2"));
        assert_eq!(transcript_support_level.unwrap(), String::from("1"));
        assert_eq!(tag.unwrap(), String::from("Ensembl_canonical"));
        assert_eq!(
            havana_transcript.unwrap(),
            String::from("OTTHUMT00000362751.1")
        );
    }

    #[test]
    fn parse_gff_line() {
        let line = "chr1\tHAVANA\ttranscript\t11869\t14409\t.\t+\t.\tID=ENST00000450305.2;Parent=ENSG00000223972.6;gene_id=ENSG00000223972.6;transcript_id=ENST00000450305.2;gene_type=transcribed_unprocessed_pseudogene;gene_name=DDX11L1;transcript_type=transcribed_unprocessed_pseudogene;transcript_name=DDX11L1-201;level=2;transcript_support_level=NA;hgnc_id=HGNC:37102;ont=PGO:0000005,PGO:0000019;tag=basic,Ensembl_canonical;havana_gene=OTTHUMG00000000961.2;havana_transcript=OTTHUMT00000002844.2".to_string();
        let attr = Attribute::parse::<b'='>(&line).unwrap();

        assert_eq!(attr.gene_id(), "ENSG00000223972.6");
        assert_eq!(attr.transcript_id(), "ENST00000450305.2");
        assert_eq!(attr.exon_number(), "z");
        assert_eq!(attr.exon_id(), "0");
    }
}
