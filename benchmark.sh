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
