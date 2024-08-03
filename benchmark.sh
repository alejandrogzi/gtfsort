#!/usr/bin/bash

set -e

COMPARE_TO="origin/master"

if [ -n "$1" ]; then
  COMPARE_TO="$1"
fi

TEST_FILE="tests/gencode.v46.chr_patch_hapl_scaff.annotation.gff3"
TEST_URL="https://ftp.ebi.ac.uk/pub/databases/gencode/Gencode_human/release_46/gencode.v46.chr_patch_hapl_scaff.annotation.gff3.gz"
STDOUT_FILE="tests/benchmark-output.txt"

if [ -z "$NUM_THREADS" ]; then
  NUM_THREADS=4
fi

mkdir -p tests

current_location=$(git rev-parse --abbrev-ref HEAD)

# shellcheck disable=SC2064
trap "git checkout '$current_location'" EXIT

BENCHMARK_TARGETS=(
  "base=${COMPARE_TO}"
  "current=${current_location}"
)

if [ ! -f "$TEST_FILE" ]; then
    curl -LsS $TEST_URL | gzip -d > $TEST_FILE
fi

join_by() { local IFS="$1"; shift; echo "$*"; }

encode_body_json() {
  node -e "const process = require('process'); const fs = require('fs'); let data = JSON.parse(process.argv[1]); const stdin = fs.readFileSync(0, 'utf-8'); data.body = stdin; console.log(JSON.stringify(data));" "$@"
}

if [ ! -f tests/sink.gff3 ] && [ ! -L tests/sink.gff3 ]; then
  ln -s /dev/null tests/sink.gff3
fi

rm -f $STDOUT_FILE

cargo clean

# shellcheck disable=SC1083,SC2016
hyperfine --warmup 3 --min-runs 5 \
    --export-csv "tests/benchmark_sink.csv" \
    --export-markdown "tests/benchmark_sink.md" \
    -L commit "$(join_by , "${BENCHMARK_TARGETS[@]}")" \
    --setup "short_name=\$(echo '{commit}' | cut -d= -f1); git checkout -B benchmark \$(echo '{commit}' | cut -d= -f2) && cargo build --release" \
    --cleanup 'cargo clean' \
    "$@" \
    "short_name=\$(echo '{commit}' | cut -d= -f1); target/release/gtfsort -i '$TEST_FILE' -o tests/sink.gff3 -t ${NUM_THREADS} 2>&1 | awk -v name=\$short_name '{ print \"[\"name\" -> sink] \" \$0 }' | tee -a '$STDOUT_FILE'"

# shellcheck disable=SC1083,SC2016
hyperfine --warmup 3 --min-runs 5 \
    --export-csv "tests/benchmark_file.csv" \
    --export-markdown "tests/benchmark_file.md" \
    -L commit "$(join_by , "${BENCHMARK_TARGETS[@]}")" \
    --setup "short_name=\$(echo '{commit}' | cut -d= -f1); git checkout -B benchmark \$(echo '{commit}' | cut -d= -f2) && cargo build --release" \
    --cleanup 'cargo clean' \
    "$@" \
    "short_name=\$(echo '{commit}' | cut -d= -f1); target/release/gtfsort -i '$TEST_FILE' -o tests/output_\${short_name}.gff3 -t ${NUM_THREADS} 2>&1 | awk -v name=\$short_name '{ print \"[\"name\" -> file] \" \$0 }' | tee -a '$STDOUT_FILE'"

if [ "$GITHUB_TOKEN" != "" ] && \
   [ "$GITHUB_REPO_NAME" != "" ] && \
   [ "$GITHUB_REPO_OWNER" != "" ] && \
   git diff --exit-code --quiet; then
  
  current_commit_sha=$(git rev-parse HEAD)

  echo "Reporting on $GITHUB_REPO_OWNER/$GITHUB_REPO_NAME:$current_commit_sha"

  {
    echo "# Benchmark results for $current_commit_sha"
    echo
    echo "## Timing Data"
    echo
    cat tests/benchmark_file.md
    echo
    echo "<details><summary>Download CSV</summary>"
    echo
    echo '```'
    cat tests/benchmark_file.csv
    echo '```'
    echo
    echo "</details>"
    echo
    echo "## Memory Usage and Logs"
    echo
    echo "<details><summary>Click to expand</summary>"
    echo
    echo '```'
    cat $STDOUT_FILE
    echo '```'
    echo
    echo "</details>"
  } | \
    encode_body_json '{"path":"benchmark.md","position":1,"line":1}' | \
      curl -L \
        -X POST \
        -H "Accept: application/vnd.github+json" \
        -H "Authorization: Bearer $GITHUB_TOKEN" \
        -H "X-GitHub-Api-Version: 2022-11-28" \
        "https://api.github.com/repos/$GITHUB_REPO_OWNER/$GITHUB_REPO_NAME/commits/$current_commit_sha/comments" \
        -d @- || echo "Failed to post benchmark results"

fi