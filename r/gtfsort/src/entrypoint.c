// We need to forward routine registration from C to Rust
// to avoid the linker removing the static library.

void R_init_gtfsort_extendr(void *dll);

void R_init_gtfsort(void *dll) {
    R_init_gtfsort_extendr(dll);
}
