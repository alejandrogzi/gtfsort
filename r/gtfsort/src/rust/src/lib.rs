use extendr_api::prelude::*;

use std::path::PathBuf;

/// Sort a GTF/GFF/GFF3 file.
///
/// @param input The input file path.
/// @param output The output file path.
/// @param threads The number of threads to use.
/// @return a list with the input and output file paths, the number of threads used, whether the input and output were memory-mapped, the time taken to parse, index, and write the output, and the memory used before and after the operation.
///
/// @examples
/// sort_annotations("tests/data/chr1.gtf", "tests/data/chr1.sorted.gtf", 1)
///
/// @export
#[extendr]
fn sort_annotations(input: &str, output: &str, threads: usize) -> Robj {
    let (input, output) = (PathBuf::from(input), PathBuf::from(output));
    match gtfsort::sort_annotations(&input, &output, threads) {
        Ok(result) => list!(
            success = true,
            input = result.input,
            output = result.output,
            threads = result.threads,
            input_mmaped = result.input_mmaped,
            output_mmaped = result.output_mmaped,
            parsing_secs = result.parsing_secs,
            indexing_secs = result.indexing_secs,
            writing_secs = result.writing_secs,
            start_mem_mb = result.start_mem_mb.unwrap_or(f64::NAN),
            end_mem_mb = result.end_mem_mb.unwrap_or(f64::NAN)
        )
        .into(),
        Err(e) => list!(success = false, error = e.to_string()).into(),
    }
}

/// Sort a string with GTF/GFF/GFF3 annotations.
///
/// @param mode The mode to parse the annotations. Either "gtf" or "gff" or "gff3".
/// @param input The string with the GTF/GFF/GFF3 annotations.
/// @param output A function that will be called with each chunk of the sorted string. Return NULL to continue, or a string to stop.
/// @param threads The number of threads to use.
/// @return a list with the input and output strings, the number of threads used, whether the input and output were memory-mapped, the time taken to parse, index, and write the output, and the memory used before and after the operation.
///
/// @examples
/// sort_annotations_str("gtf", "chr1\t.\texon\t11869\t12227\t.\t+\t.\tgene_id \"ENSG00000223972.5\"; transcript_id \"ENST00000456328.2\"; exon_number \"1\";\nchr1\t.\texon\t12613\t12721\t.\t+\t.\tgene_id \"ENSG00000223972.5\"; transcript_id \"ENST00000456328.2\"; exon_number \"2\";", function(str) { cat(str); return(NULL); }, 1)
///
/// @export
#[extendr]
fn sort_annotations_str(mode: &str, input: &str, output: Robj, threads: usize) -> Robj {
    let Some(output) = output.as_function() else {
        return list!(success = false, error = "output must be a function").into();
    };

    let input = input.to_string();

    let mut err = None;

    let mut output = |str: &[u8]| {
        let ret = output.call(pairlist!(str)).unwrap();
        match ret.is_null() {
            true => Ok(str.len()),
            false => {
                let e = Err(std::io::Error::other(ret.as_str().unwrap().to_string()));
                err = Some(ret);
                e
            }
        }
    };

    let result = match mode {
        "gtf" => gtfsort::sort_annotations_string::<b' ', _>(&input, &mut output, threads),
        "gff" | "gff3" => gtfsort::sort_annotations_string::<b'=', _>(&input, &mut output, threads),
        _ => {
            return list!(
                success = false,
                error = "mode must be 'gtf', 'gff', or 'gff3'"
            )
            .into()
        }
    };

    match result {
        Ok(result) => list!(
            success = true,
            input = result.input,
            output = result.output,
            threads = result.threads,
            input_mmaped = result.input_mmaped,
            output_mmaped = result.output_mmaped,
            parsing_secs = result.parsing_secs,
            indexing_secs = result.indexing_secs,
            writing_secs = result.writing_secs,
            start_mem_mb = result.start_mem_mb.unwrap_or(f64::NAN),
            end_mem_mb = result.end_mem_mb.unwrap_or(f64::NAN)
        )
        .into(),
        Err(e) => list!(success = false, error = e.to_string()).into(),
    }
}

extendr_module! {
    mod gtfsort;
    fn sort_annotations;
    fn sort_annotations_str;
}
