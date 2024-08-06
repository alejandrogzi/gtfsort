use clap::Parser;
use flate2::read::GzDecoder;
use reqwest::blocking::{get, ClientBuilder};
use std::env;
use std::fs::File;
use std::path::Path;
use std::process::{Command, ExitStatus, Stdio};

const TEST_FILE: &str = "tests/gencode.v46.chr_patch_hapl_scaff.annotation.gff3";
const TEST_URL: &str = "https://ftp.ebi.ac.uk/pub/databases/gencode/Gencode_human/release_46/gencode.v46.chr_patch_hapl_scaff.annotation.gff3.gz";
const STDOUT_FILE: &str = "tests/benchmark-output.txt";

const TARGET_EXEC: &str = if cfg!(windows) {
    "target/release/gtfsort.exe"
} else {
    "target/release/gtfsort"
};

#[derive(Debug, Parser)]
pub struct Args {
    #[clap(
        short = 'r',
        long = "reference",
        help = "Reference commit to compare against",
        default_value = "origin/master"
    )]
    compare_to: String,
    #[clap(
        short = 't',
        long = "threads",
        help = "Number of threads to use",
        default_value = "4"
    )]
    threads: u32,
    #[clap(help = "Extra arguments to pass to hyperfine")]
    hyperfine_args: Vec<String>,
}

pub struct HyperfineCall {
    pub warmup: u32,
    pub min_runs: u32,
    pub export_csv: Option<String>,
    pub export_markdown: Option<String>,
    pub parameters: Vec<(String, Vec<String>)>,
    pub setup: Option<String>,
    pub cleanup: Option<String>,
    pub command: String,
    pub extras: Vec<String>,
}

fn run_git(args: &[&str]) -> Result<String, ExitStatus> {
    let output = Command::new("git")
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .stdin(Stdio::null())
        .output()
        .expect("Failed to run git");

    if output.status.success() {
        Ok(String::from_utf8(output.stdout).expect("Failed to parse git output"))
    } else {
        Err(output.status)
    }
}

impl Default for HyperfineCall {
    fn default() -> Self {
        Self {
            warmup: 3,
            min_runs: 5,
            export_csv: None,
            export_markdown: None,
            parameters: Vec::new(),
            setup: None,
            cleanup: None,
            command: String::new(),
            extras: Vec::new(),
        }
    }
}

impl HyperfineCall {
    pub fn invoke(&self) -> ExitStatus {
        let mut command = Command::new("hyperfine");

        command
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .stdin(Stdio::null());

        command.arg("--warmup").arg(self.warmup.to_string());
        command.arg("--min-runs").arg(self.min_runs.to_string());
        if let Some(export_csv) = &self.export_csv {
            command.arg("--export-csv").arg(export_csv);
        }
        if let Some(export_markdown) = &self.export_markdown {
            command.arg("--export-markdown").arg(export_markdown);
        }
        for (flag, values) in &self.parameters {
            command.arg("-L").arg(flag).arg(values.join(","));
        }
        if let Some(setup) = &self.setup {
            command.arg("--setup").arg(setup);
        }
        if let Some(cleanup) = &self.cleanup {
            command.arg("--cleanup").arg(cleanup);
        }
        if !self.extras.is_empty() {
            command.args(&self.extras);
        }
        command.arg(&self.command);

        command.status().expect("Failed to run hyperfine")
    }
}

fn report_to_github(
    result: Result<(String, String), Box<dyn std::error::Error>>,
    stdout: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    if run_git(&["diff", "--quiet"]).is_err() {
        eprintln!("Working directory is dirty, skipping GitHub comment");
        return Ok(());
    }

    let current_commit_sha = run_git(&["rev-parse", "HEAD"])
        .map(|s| s.trim().to_string())
        .unwrap();

    if let Ok((token, repo_name, repo_owner)) = env::var("GITHUB_TOKEN").and_then(|token| {
        env::var("GITHUB_REPO_NAME").and_then(|repo_name| {
            env::var("GITHUB_REPO_OWNER").map(|repo_owner| (token, repo_name, repo_owner))
        })
    }) {
        let mut body = String::new();

        body.push_str(format!("# Benchmark results on {} for:", std::env::consts::OS).as_str());
        body.push_str(&current_commit_sha);

        match result {
            Ok((timing_md, timing_csv)) => {
                body.push_str("\n\n## Timing Data\n\n");
                body.push_str(
                    &std::fs::read_to_string(timing_md).expect("Failed to read timing markdown"),
                );
                body.push_str("\n\n<details><summary>Download CSV</summary>\n\n```");
                body.push_str(
                    &std::fs::read_to_string(timing_csv).expect("Failed to read timing CSV"),
                );
                body.push_str("```\n\n</details>\n\n## Memory Usage and Logs\n\n<details><summary>Click to expand</summary>\n\n```");
                body.push_str(
                    &std::fs::read_to_string(stdout).unwrap_or("Failed to read stdout".to_string()),
                );
                body.push_str("```\n\n</details>");
            }
            Err(err) => {
                body.push_str("\n\n## Error\n\n");
                body.push_str(&format!("{:?}", err));
                body.push_str("\n\n<details><summary>Full Output</summary>\n\n```");
                body.push_str(
                    &std::fs::read_to_string(stdout).unwrap_or("Failed to read stdout".to_string()),
                );
                body.push_str("```\n\n</details>");
            }
        }

        #[derive(serde::Serialize)]
        struct Body {
            pub body: String,
        }

        let client = ClientBuilder::new()
            .user_agent("gtfsort-benchmark")
            .build()
            .expect("Failed to create reqwest client");

        let res = client
            .post(format!(
                "https://api.github.com/repos/{}/{}/commits/{}/comments",
                repo_owner, repo_name, current_commit_sha
            ))
            .header("Accept", "application/vnd.github+json")
            .bearer_auth(token)
            .header("X-GitHub-Api-Version", "2022-11-28")
            .body(serde_json::to_string(&Body { body }).expect("Failed to serialize body"))
            .send()?;

        if !res.status().is_success() {
            return Err(format!("Failed to post comment to GitHub: {}", res.text()?).into());
        }
    }

    Ok(())
}

fn benchmark() -> Result<(String, String), Box<dyn std::error::Error>> {
    let args = Args::parse();

    let num_threads = args.threads;

    std::fs::create_dir_all("tests")?;

    let test_file = Path::new(TEST_FILE);

    if !test_file.exists() {
        let mut response = GzDecoder::new(get(TEST_URL)?);
        let mut file = File::create(test_file)?;

        std::io::copy(&mut response, &mut file)?;
    }

    let current_location = run_git(&["rev-parse", "--abbrev-ref", "HEAD"])
        .map(|s| s.trim().to_string())
        .expect("Failed to get current commit");

    #[allow(clippy::needless_update)]
    let code =  HyperfineCall {
        warmup: 3,
        min_runs: 5,
        export_csv: Some("tests/benchmark_file.csv".to_string()),
        export_markdown: Some("tests/benchmark_file.md".to_string()),
        parameters: vec![("commit".to_string(), vec![format!("ref={}", args.compare_to), format!("this={}", current_location)])],
        setup: Some("short_name=$(echo '{commit}' | cut -d= -f1); git checkout -B benchmark $(echo '{commit}' | cut -d= -f2) && cargo build --release".to_string()),
        cleanup: Some("cargo clean".to_string()),
        command: format!("short_name=$(echo '{{commit}}' | cut -d= -f1); {} -i '{}' -o tests/output_${{short_name}}.gff3 -t {} 2>&1 | awk -v name=$short_name '{{ print \"[\"name\" -> file] \" $0 }}' | tee -a '{}'", TARGET_EXEC, TEST_FILE, num_threads, STDOUT_FILE),
        extras: args.hyperfine_args,
        ..Default::default()
    }.invoke().code().expect("Benchmark terminated unexpectedly");

    run_git(&["checkout", &current_location]).expect("Failed to checkout to original commit");

    if code != 0 {
        return Err(format!("Benchmark failed with exit code {}", code).into());
    }

    Ok((
        "tests/benchmark_file.md".to_string(),
        "tests/benchmark_file.csv".to_string(),
    ))
}

fn main() {
    let stdout = Path::new(STDOUT_FILE);

    let result = benchmark();

    if let Err(err) = report_to_github(result, stdout) {
        eprintln!("Failed to report to GitHub: {:?}", err);
    }
}
