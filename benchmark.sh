#!/usr/bin/bash

set -e

TEST_FILE="tests/gencode.v46.chr_patch_hapl_scaff.annotation.gff3"
TEST_URL="https://ftp.ebi.ac.uk/pub/databases/gencode/Gencode_human/release_46/gencode.v46.chr_patch_hapl_scaff.annotation.gff3.gz"
BENCHMARK_TARGETS=(
  "base=48362e13d651be64e4f1f10c725519973dc8dc9a"
  "nocopy=fa29008f7e1924c377d83d0d1247b1cf46c40f39"
  "mmap=dce6085093662c99f9b0fe1e1447c005b84f48fd"
  "mmapwrite=b2869cbd630584c28a305a6b5e61a863acd242c0"
)
STDOUT_FILE="tests/benchmark-output.txt"

mkdir -p tests

previous_location=$(git rev-parse --abbrev-ref HEAD)

# shellcheck disable=SC2064
trap "git checkout '$previous_location'" EXIT

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
    "short_name=\$(echo '{commit}' | cut -d= -f1); target/release/gtfsort -i '$TEST_FILE' -o tests/sink.gff3 -t 16 2>&1 | awk -v name=\$short_name '{ print \"[\"name\" -> sink] \" \$0 }' | tee -a '$STDOUT_FILE'"

# shellcheck disable=SC1083,SC2016
hyperfine --warmup 3 --min-runs 5 \
    --export-csv "tests/benchmark_file.csv" \
    --export-markdown "tests/benchmark_file.md" \
    -L commit "$(join_by , "${BENCHMARK_TARGETS[@]}")" \
    --setup "short_name=\$(echo '{commit}' | cut -d= -f1); git checkout -B benchmark \$(echo '{commit}' | cut -d= -f2) && cargo build --release" \
    --cleanup 'cargo clean' \
    "$@" \
    "short_name=\$(echo '{commit}' | cut -d= -f1); target/release/gtfsort -i '$TEST_FILE' -o tests/output_\${short_name}.gff3 -t 16 2>&1 | awk -v name=\$short_name '{ print \"[\"name\" -> file] \" \$0 }' | tee -a '$STDOUT_FILE'"
