#[cfg(feature = "c_ffi")]
pub mod c_ffi {
    use crate::{GtfSortError, SortAnnotationsJobResult};

    use std::ffi::{c_char, c_ulong, c_void, CStr, CString};

    #[repr(C)]
    pub struct GtfSortErrorFFI {
        pub code: i32,
        pub message: *const c_char,
    }

    pub const GTFSORT_ERROR_INVALID_INPUT: i32 = 1;
    pub const GTFSORT_ERROR_INVALID_OUTPUT: i32 = 2;
    pub const GTFSORT_ERROR_PARSE_ERROR: i32 = 3;
    pub const GTFSORT_ERROR_INVALID_THREADS: i32 = 4;
    pub const GTFSORT_ERROR_IO_ERROR: i32 = 5;
    pub const GTFSORT_ERROR_INVALID_PARAMETER: i32 = -1;

    macro_rules! cstr {
        ($s:expr) => {
            CString::new($s).unwrap().into_raw()
        };
    }

    macro_rules! cstr_free {
        ($s:expr) => {
            if !$s.is_null() {
                drop(CString::from_raw($s as *mut _));
            }
        };
    }

    impl From<GtfSortError> for GtfSortErrorFFI {
        fn from(e: GtfSortError) -> Self {
            match e {
                GtfSortError::InvalidInput(s) => Self {
                    code: GTFSORT_ERROR_INVALID_INPUT,
                    message: cstr!(s),
                },
                GtfSortError::InvalidOutput(s) => Self {
                    code: GTFSORT_ERROR_INVALID_OUTPUT,
                    message: cstr!(s),
                },
                GtfSortError::ParseError(s) => Self {
                    code: GTFSORT_ERROR_PARSE_ERROR,
                    message: cstr!(s),
                },
                GtfSortError::InvalidThreads(s) => Self {
                    code: GTFSORT_ERROR_INVALID_THREADS,
                    message: cstr!(s),
                },
                GtfSortError::IoError(s, e) => Self {
                    code: GTFSORT_ERROR_IO_ERROR,
                    message: cstr!(format!("{}: {}", s, e)),
                },
                GtfSortError::InvalidParameter(s) => Self {
                    code: GTFSORT_ERROR_INVALID_PARAMETER,
                    message: cstr!(s),
                },
            }
        }
    }

    #[repr(C)]
    pub struct SortAnnotationsJobResultFFI {
        pub input: *const c_char,
        pub output: *const c_char,
        pub threads: usize,
        pub input_mmaped: bool,
        pub output_mmaped: bool,
        pub parsing_secs: f64,
        pub indexing_secs: f64,
        pub writing_secs: f64,
        pub start_mem_mb: f64,
        pub end_mem_mb: f64,
    }

    impl From<SortAnnotationsJobResult<'_>> for SortAnnotationsJobResultFFI {
        fn from(r: SortAnnotationsJobResult) -> Self {
            Self {
                input: cstr!(r.input),
                output: cstr!(r.output),
                threads: r.threads,
                input_mmaped: r.input_mmaped,
                output_mmaped: r.output_mmaped,
                parsing_secs: r.parsing_secs,
                indexing_secs: r.indexing_secs,
                writing_secs: r.writing_secs,
                start_mem_mb: r.start_mem_mb.unwrap_or(f64::NAN),
                end_mem_mb: r.end_mem_mb.unwrap_or(f64::NAN),
            }
        }
    }

    #[repr(C)]
    pub enum SortAnnotationsRet {
        Ok(*mut SortAnnotationsJobResultFFI),
        Err(*mut GtfSortErrorFFI),
    }

    const GTFSORT_PARSE_MODE_GTF: u8 = 1;
    const GTFSORT_PARSE_MODE_GFF3: u8 = 2;

    #[no_mangle]
    unsafe extern "C" fn gtfsort_init_logger(level: *const c_char) {
        let level = unsafe { CStr::from_ptr(level).to_str().unwrap_or("info") };
        match level.to_ascii_lowercase().as_str() {
            "trace" => simple_logger::init_with_level(log::Level::Trace).unwrap(),
            "debug" => simple_logger::init_with_level(log::Level::Debug).unwrap(),
            "info" => simple_logger::init_with_level(log::Level::Info).unwrap(),
            "warn" => simple_logger::init_with_level(log::Level::Warn).unwrap(),
            "error" => simple_logger::init_with_level(log::Level::Error).unwrap(),
            _ => simple_logger::init_with_level(log::Level::Info).unwrap(),
        }
    }

    #[no_mangle]
    unsafe extern "C" fn gtfsort_new_sort_annotations_ret() -> *mut SortAnnotationsRet {
        Box::into_raw(Box::new(SortAnnotationsRet::Ok(std::ptr::null_mut())))
    }

    #[no_mangle]
    unsafe extern "C" fn gtfsort_free_sort_annotations_ret(ret: SortAnnotationsRet) {
        match ret {
            SortAnnotationsRet::Ok(ptr) => unsafe {
                cstr_free!((*ptr).input);
                cstr_free!((*ptr).output);
                drop(Box::from_raw(ptr));
            },
            SortAnnotationsRet::Err(ptr) => unsafe {
                cstr_free!((*ptr).message);
                drop(Box::from_raw(ptr));
            },
        }
    }

    #[no_mangle]
    unsafe extern "C" fn gtfsort_sort_annotations(
        input: *const std::os::raw::c_char,
        output: *const std::os::raw::c_char,
        threads: usize,
        result_ptr: *mut SortAnnotationsRet,
    ) -> bool {
        let input = std::path::PathBuf::from(unsafe { CStr::from_ptr(input).to_str().unwrap() });
        let output = std::path::PathBuf::from(unsafe { CStr::from_ptr(output).to_str().unwrap() });

        let result = crate::sort_annotations(&input, &output, threads);

        let ok = result.is_ok();

        if !result_ptr.is_null() {
            unsafe {
                *result_ptr = match result {
                    Ok(r) => SortAnnotationsRet::Ok(Box::into_raw(Box::new(r.into()))),
                    Err(e) => SortAnnotationsRet::Err(Box::into_raw(Box::new(e.into()))),
                };
            }
        }

        ok
    }

    #[no_mangle]
    unsafe extern "C" fn gtfsort_sort_annotations_gtf_str(
        mode: u8,
        input: *const c_char,
        output: extern "C" fn(*mut c_void, *const c_char, c_ulong) -> *const c_char,
        threads: usize,
        caller_data: *mut c_void,
        result_ptr: *mut SortAnnotationsRet,
    ) -> bool {
        let input = unsafe { CStr::from_ptr(input).to_str().unwrap() };

        let mut output = |str: &[u8]| {
            let ret = output(
                caller_data,
                unsafe { CStr::from_bytes_with_nul_unchecked(str).as_ptr() },
                str.len() as c_ulong,
            );
            match ret.is_null() {
                true => Ok(str.len()),
                false => Err(std::io::Error::other(unsafe {
                    CStr::from_ptr(ret).to_str().unwrap()
                })),
            }
        };

        let result = match mode {
            GTFSORT_PARSE_MODE_GTF => {
                crate::sort_annotations_string::<b' ', _>(input, &mut output, threads)
            }
            GTFSORT_PARSE_MODE_GFF3 => {
                crate::sort_annotations_string::<b'=', _>(input, &mut output, threads)
            }
            _ => {
                unsafe {
                    *result_ptr = SortAnnotationsRet::Err(Box::into_raw(Box::new(
                        GtfSortError::InvalidParameter("invalid parse mode").into(),
                    )));
                }
                return false;
            }
        };

        let ok = result.is_ok();

        if !result_ptr.is_null() {
            unsafe {
                *result_ptr = match result {
                    Ok(r) => SortAnnotationsRet::Ok(Box::into_raw(Box::new(r.into()))),
                    Err(e) => SortAnnotationsRet::Err(Box::into_raw(Box::new(e.into()))),
                };
            }
        }

        ok
    }
}
