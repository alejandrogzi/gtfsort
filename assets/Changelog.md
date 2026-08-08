# Changelog

## 0.2.6 - 2026-08-08

- `dd7e9bb` feat: streamable I/O via stdin/stdout + gzip + v0.2.6 bump — alejandrogzi
- `e44b8b4` docs: README streamable I/O usage — alejandrogzi

## 0.2.5 - 2026-07-21

- `c8ae850` BREAKING CHANGE: v0.2.5 -> preserve metadata lines + docs + changelog — alejandrogzi
- `5e30cc7` BREAKING CHANGE: v0.2.5 -> container — alejandrogzi
- `8ccdbe0` fix: update checksum for M35 — alejandrogzi

## 0.2.4 - 2026-03-16

- `5feb5f6` Merge pull request #18 from alejandrogzi/v0.2.4 — Alejandro Gonzales-Irribarren
- `d684767` fix: include pandas in py-port env — alejandrogzi
- `9012d5f` fix: comply with windows mmap handler; switch to uv from hatch for py-port test — alejandrogzi
- `af075d1` chore: update lock — alejandrogzi
- `d6bb32b` fix: transcript blocks + repeated child feats re-sorted differently; deterministic test output — alejandrogzi
- `0702d89` BREAKING CHANGE: v0.2.4 -> drop sorter dependency on gene + gene/transcript block structs + unify sequential/mmap writers — alejandrogzi

## 0.2.3 - 2025-04-27

- `baef012` chore: README path — alejandrogzi
- `2db270e` fix: catching lines without transcript_ids — alejandrogzi
- `ec260ae` Merge pull request #12 from eternal-flame-AD/null-pointer — Alejandro Gonzales-Irribarren
- `33353a8` Fix a null pointer dereference in C API gtfsort_sort_annotations_gtf_str() — eternal-flame-AD
- `b6f5e01` chore: update README — alejandrogzi
- `fcf7062` chore: update benches — alejandrogzi
- `f5b6c50` Merge pull request #10 from alejandrogzi/py-gtfsort — Alejandro Gonzales-Irribarren
- `0bd745b` chore: partial README update — alejandrogzi
- `87fc993` fix: drop maturin checkout -> impl pipx + develop — alejandrogzi
- `ec7ae6a` fix: complete py port test mod — alejandrogzi
- `3fbe320` chore: disable win need for py port — alejandrogzi
- `7b5743a` fix: py version — alejandrogzi
- `e903236` chore: small refactoring — alejandrogzi
- `5ba38c0` feat: port part of integration test to python module [.py incomplete -> need to assert sorting] — alejandrogzi
- `8679529` fix: check tmp is downloaded — alejandrogzi
- `83c6e77` fix: move rs-ci to ensure workflow success — alejandrogzi
- `abc2a77` feat: updating CI to include wheels — alejandrogzi
- `27235c8` chore: just renaming stuff — alejandrogzi
- `f46cac3` chore: small fixes — alejandrogzi
- `9986239` fix: oops forgot to move gitignore and cover target — alejandrogzi
- `864901e` chore: move rs own tree — alejandrogzi
- `16afd0b` .gitignore — alejandrogzi
- `7d04494` chore: move own rs tree — alejandrogzi
- `ef9f973` BREAKING CHANGE: Py port of gtfsort! — alejandrogzi
- `92c1f96` chore: update cite — alejandrogzi
- `8fc0d89` Merge pull request #9 from eternal-flame-AD/master — Alejandro Gonzales-Irribarren
- `fa9e118` Fix valgrind error when reusing SortAnnotationsRet — eternal-flame-AD
- `858d0b4` Remove unused dependencies — eternal-flame-AD
- `96d67a2` Get rid of openssl dependency on release binary — eternal-flame-AD
- `a0afe6c` Document C API — eternal-flame-AD
- `7d239be` fixup! Upload build artifacts — eternal-flame-AD
- `77719b7` Add default-run in Cargo.toml — eternal-flame-AD
- `1bc9eac` Upload build artifacts — eternal-flame-AD
- `57651c3` [ci benchmark] Merge benchmark flow into check — eternal-flame-AD
- `0146f32` Don't comment on specific location on benchmark — eternal-flame-AD
- `12f61f0` Finish C FFI — eternal-flame-AD
- `3acfef5` Cross platform benchmark — eternal-flame-AD
- `ce56b62` Auto report benchmark results — eternal-flame-AD
- `c9f98aa` CI interops and mmap fixes — eternal-flame-AD
- `7fa3b1c` Merge pull request #8 from eternal-flame-AD/faster — Alejandro Gonzales-Irribarren
- `1161283` Update benchmark script — eternal-flame-AD
- `b31cf26` fallback to sequential on mmap() fail — eternal-flame-AD
- `f6583b3` remove allocation on Record::parse — eternal-flame-AD
- `06a6fde` Upload benchmark script — eternal-flame-AD
- `b2869cb` revert trying to parallelize index building — eternal-flame-AD
- `349e745` fix imports without mmap — eternal-flame-AD
- `7b7cc4a` Detailed timing — eternal-flame-AD
- `3b50cab` use mmaped file write — eternal-flame-AD
- `dce6085` Implement memory mapped read — eternal-flame-AD
- `fa29008` reduce copying and cloning — eternal-flame-AD
- `48362e1` fix time-rs/time#681 — eternal-flame-AD

## 0.2.2 - 2024-03-01

- `50effe8` BREAKING CHANGE: v.0.2.2, GFF support — alejandrogzi
- `bc41626` feat: Layers struct + ChromRecords out of parallel parsing — alejandrogzi
- `c2b451d` feat: new efficient helper fn -> outer-inner layers — alejandrogzi
- `f884e84` feat: GFF support -> general parsing approach — alejandrogzi
- `dd0921b` README — alejandrogzi
- `ceeb171` cargo — alejandrogzi
- `3577904` README — alejandrogzi
- `835ead6` README — alejandrogzi
- `e97bf35` feat: container — alejandrogzi
- `bde7313` CITATION — alejandrogzi
- `fb8b59d` README — alejandrogzi

## 0.2.1 - 2023-12-16

- `e98d151` README — alejandrogzi
- `1bb5848` BREAKING CHANGE: parallel v.0.2.1; fix error printing + hash calling — alejandrogzi
- `78eec66` Merge pull request #5 from alejandrogzi/parallel_sort — Alejandro Gonzales-Irribarren
- `56d98c9` BREAKING CHANGE: now parallel gtfsort; v.0.2.1 — alejandrogzi
- `3d7135e` gitignore — alejandrogzi
- `e2897e5` cargo — alejandrogzi
- `f65af57` feat(): Record::parse adapted for para workers — alejandrogzi
- `7be2675` BREAKING CHANGE: v.0.2.1 cli-only partial use — alejandrogzi
- `31b9da6` nw(): parallel handlers v.0.2.1 — alejandrogzi

## 0.1.1 - 2023-10-19

- `9a6e5c7` Merge pull request #1 from alejandrogzi/splitb_gencode_bug — Alejandro Gonzales-Irribarren
- `473f795` fix(splitb): fix byte-splitting to handle unquoted features in GENCODE files — alejandrogzi
- `2485cbc` fix(sx): fix syntax/typos; v.0.1.1 — alejandrogzi
- `50c0969` cargo — alejandrogzi
- `75329b7` README — alejandrogzi
- `2979c49` README — alejandrogzi
- `b01bf37` README — alejandrogzi
- `be4581a` README — alejandrogzi
- `0b27142` fix(about): update about/help args — alejandrogzi
- `730461c` cargo — alejandrogzi
- `cb882c6` init(supp): benchmark, nodes — alejandrogzi
- `b227f84` README — alejandrogzi
- `b4a8261` new(supp): init commit; overview — alejandrogzi
- `f42bebc` fix(layer): change cmp -> natord:compare for outer layer natural sorting — alejandrogzi
- `a42c727` upd(msg): add final CLI message — alejandrogzi
- `7f3f623` fix(misc; test): implement miscellanous layer; inter/intra chromosomal sorting tests — alejandrogzi
- `14aa865` fix(attr): handle parsing error outside Attribute struct; update tests — alejandrogzi
- `e34e0cf` fix(test); impl(misc): updated tests; implement miscellanous layer outside of exon/CDS to handle multiple UTR/Selenocysteine records — alejandrogzi
- `563c615` fix(test): update/pass inner/outer tests w/ diff formats — alejandrogzi
- `88bfbe2` upd(test): inner/outer/empty module tests using Mus musculus snippets — alejandrogzi
- `1de1078` fix(new): change empty checking logic — alejandrogzi
- `27dd1da` README — alejandrogzi
- `9d35899` README — alejandrogzi
- `0f4f109` upd(log): add intro + logging info — alejandrogzi
- `6288f91` fix(ParseError): derive PartialEq — alejandrogzi
- `c29fc0f` fix(use): update imports — alejandrogzi
- `bbffab7` cargo — alejandrogzi
- `d6f8b23` fix(sort_by): updates 2-key sort of chr/pos; does not increase time — alejandrogzi
- `4ea61e2` upd(main): reduce args; calls sorter — alejandrogzi
- `37980f0` BREAKING CHANGE: minimalist version; reduced x3 computation time — alejandrogzi
- `660969e` BREAKING CHANGE: optimized structures; byte-slicing approach — alejandrogzi
- `0a59f7b` cargo — alejandrogzi
- `a6112d4` init(attr): byte-slicing retrieve of attributes — alejandrogzi
- `d7fd4d4` init(ord): BTreeMap sort impl for exons — alejandrogzi
- `5d8122f` fix(line_parser): unnecessary statements — alejandrogzi
- `d2588eb` fix(parallel): pass chunks to par_chunks; ~2s optimization — alejandrogzi
- `b9b17cc` fix(): typos — alejandrogzi
- `cdb6b5f` cargo — alejandrogzi
- `f2f5d26` fix(parallel): parallel parsing; logging layer + peak_alloc info — alejandrogzi
- `85f0b9b` feat(args): support -cpu; default max_cpus — alejandrogzi
- `66643c0` fix(attr): fix typos — alejandrogzi
- `a6fd50f` CARGO — alejandrogzi
- `a2c8ed0` LICENSE — alejandrogzi
- `5c94a3b` init commit — alejandrogzi
- `d922c38` gitignore — alejandrogzi
- `14d9e99` cargo init commit — alejandrogzi

