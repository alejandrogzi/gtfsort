#![allow(dead_code)]

use std::{fs::File, path::PathBuf};

use gtfsort::{current_func, sort_annotations, sort_annotations_string, test_utils::*};

fn test_gencode_m35_subset_with_n_threads(nthreads: usize, prevent_mmap: bool) {
    ensure_logger_initialized();

    let test_file = get_test_file_gff3_gencode_mouse_m35();

    if prevent_mmap {
        test_file.execute_test(current_func!(), |s| {
            let input_str = std::fs::read_to_string(s).unwrap();
            let mut output_buf = Vec::new();

            let job_info = sort_annotations_string::<b'=', _>(
                &input_str,
                &mut |b| {
                    output_buf.extend_from_slice(b);

                    Ok(b.len())
                },
                nthreads,
            )
            .expect("Failed to sort annotations");

            assert_eq!(job_info.threads, nthreads);

            #[allow(clippy::bool_assert_comparison)]
            {
                assert_eq!(job_info.input_mmaped, false);
                assert_eq!(job_info.output_mmaped, false);
            }

            assert!(job_info.end_mem_mb.unwrap().is_sign_positive());
            assert!(job_info.start_mem_mb.unwrap().is_sign_positive());

            crc32_hex(&output_buf[..])
        });
    } else {
        test_file.execute_test(current_func!(), |s| {
            let input = PathBuf::from(s);
            let tmp = TempFile::new(
                format!(
                    "{}_{}_{}.sorted",
                    input.file_stem().unwrap().to_str().unwrap(),
                    nthreads,
                    current_func!().replace(|c: char| !c.is_alphanumeric(), "_")
                )
                .as_str(),
                true,
            );

            let job_info =
                sort_annotations(&input, &tmp, nthreads).expect("Failed to sort annotations");

            assert_eq!(job_info.threads, nthreads);

            #[allow(clippy::bool_assert_comparison)]
            {
                #[cfg(feature = "mmap")]
                {
                    assert_eq!(job_info.input_mmaped, true);
                    assert_eq!(job_info.output_mmaped, true);
                }
                #[cfg(not(feature = "mmap"))]
                {
                    assert_eq!(job_info.input_mmaped, false);
                    assert_eq!(job_info.output_mmaped, false);
                }
            }

            assert!(job_info.end_mem_mb.unwrap().is_sign_positive());
            assert!(job_info.start_mem_mb.unwrap().is_sign_positive());

            crc32_hex(File::open(&*tmp).unwrap())
        });
    }
}

#[test]
fn test_gencode_m35_subset_single_thread() {
    test_gencode_m35_subset_with_n_threads(1, false);
}

#[test]
fn test_gencode_m35_subset_max_threads() {
    test_gencode_m35_subset_with_n_threads(num_cpus::get(), false);
}

#[test]
#[cfg(feature = "mmap")]
fn test_gencode_m35_subset_prevent_mmap_single_thread() {
    test_gencode_m35_subset_with_n_threads(1, true);
}

#[test]
#[cfg(feature = "mmap")]
fn test_gencode_m35_subset_prevent_mmap_max_threads() {
    test_gencode_m35_subset_with_n_threads(num_cpus::get(), true);
}
