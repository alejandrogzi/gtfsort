use flate2::{read::GzDecoder, write::GzEncoder, Compression};
use std::{
    ffi::OsString,
    fs::File,
    io::{Read, Write},
    path::PathBuf,
    process::{Command, Output, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

const GTF_INPUT: &str = "\
chr2\tsrc\tgene\t20\t30\t.\t+\t.\tgene_id \"G2\";\n\
chr1\tsrc\tgene\t10\t20\t.\t+\t.\tgene_id \"G1\";\n";

const GTF_SORTED: &str = "\
chr1\tsrc\tgene\t10\t20\t.\t+\t.\tgene_id \"G1\";\n\
chr2\tsrc\tgene\t20\t30\t.\t+\t.\tgene_id \"G2\";\n";

fn run_gtfsort(args: &[OsString], input: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_gtfsort"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();

    child.wait_with_output().unwrap()
}

fn temp_path(suffix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("gtfsort_cli_{nanos}.{suffix}"))
}

#[test]
fn omitted_paths_pipe_gtf_without_polluting_stdout() {
    let output = run_gtfsort(&[], GTF_INPUT);

    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout).unwrap(), GTF_SORTED);
    assert!(String::from_utf8(output.stderr)
        .unwrap()
        .contains("GTF file sorted successfully"));
}

#[test]
fn dash_paths_pipe_inferred_gff3() {
    let input = "\
##gff-version 3\n\
chr2\tsrc\tgene\t20\t30\t.\t+\t.\tgene_id=G2;\n\
chr1\tsrc\tgene\t10\t20\t.\t+\t.\tgene_id=G1;\n";
    let expected = "\
##gff-version 3\n\
chr1\tsrc\tgene\t10\t20\t.\t+\t.\tgene_id=G1;\n\
chr2\tsrc\tgene\t20\t30\t.\t+\t.\tgene_id=G2;\n";
    let output = run_gtfsort(&["-i".into(), "-".into(), "-o".into(), "-".into()], input);

    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout).unwrap(), expected);
}

#[test]
fn streamed_side_retains_named_gzip_support() {
    let input_path = temp_path("gtf.gz");
    let output_path = temp_path("gtf.gz");

    let mut encoder = GzEncoder::new(File::create(&input_path).unwrap(), Compression::default());
    encoder.write_all(GTF_INPUT.as_bytes()).unwrap();
    encoder.finish().unwrap();

    let output = run_gtfsort(&["-i".into(), input_path.clone().into()], "");
    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout).unwrap(), GTF_SORTED);

    let output = run_gtfsort(&["-o".into(), output_path.clone().into()], GTF_INPUT);
    assert!(output.status.success());
    assert!(output.stdout.is_empty());

    let mut sorted = String::new();
    GzDecoder::new(File::open(&output_path).unwrap())
        .read_to_string(&mut sorted)
        .unwrap();
    assert_eq!(sorted, GTF_SORTED);

    std::fs::remove_file(input_path).unwrap();
    std::fs::remove_file(output_path).unwrap();
}

#[test]
fn empty_stdin_is_rejected() {
    let output = run_gtfsort(&[], "");

    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8(output.stderr)
        .unwrap()
        .contains("stdin is empty"));
}

#[test]
fn stream_failures_do_not_leak_diagnostics_to_stdout() {
    let malformed = "chr1\tsrc\tgene\t10\t20\t.\t+\t.\ttranscript_id \"T1\";\n";
    let output = run_gtfsort(&[], malformed);

    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8(output.stderr)
        .unwrap()
        .contains("Missing gene_id"));

    let output_path = temp_path("gtf");
    std::fs::create_dir(&output_path).unwrap();
    let output = run_gtfsort(&["-o".into(), output_path.clone().into()], GTF_INPUT);

    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8(output.stderr)
        .unwrap()
        .contains("creating output file"));

    std::fs::remove_dir(output_path).unwrap();
}
