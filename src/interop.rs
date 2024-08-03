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

    pub const GTFSORT_PARSE_MODE_GTF: u8 = 1;
    pub const GTFSORT_PARSE_MODE_GFF: u8 = 2;
    pub const GTFSORT_PARSE_MODE_GFF3: u8 = 2;

    /// Initializes the logger with the given log level.
    /// The log level must be one of the following: trace, debug, info, warn, error.
    ///
    /// # Safety
    /// level must be a valid C string.
    #[no_mangle]
    pub unsafe extern "C" fn gtfsort_init_logger(level: *const c_char) {
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

    /// Allocates a new [SortAnnotationsRet] on the Rust heap.
    ///
    /// # Safety
    /// The caller is responsible for freeing the allocated memory using [gtfsort_free_sort_annotations_ret].
    /// Do not free the memory using any other method.
    #[no_mangle]
    pub unsafe extern "C" fn gtfsort_new_sort_annotations_ret() -> *mut SortAnnotationsRet {
        Box::into_raw(Box::new(SortAnnotationsRet::Ok(std::ptr::null_mut())))
    }

    /// Frees the [SortAnnotationsRet].
    ///
    /// # Safety
    /// ret must be a valid pointer to a [SortAnnotationsRet] that is allocated by [gtfsort_new_sort_annotations_ret].
    #[no_mangle]
    pub unsafe extern "C" fn gtfsort_free_sort_annotations_ret(ret: *mut SortAnnotationsRet) {
        let b = Box::from_raw(ret);

        match *b {
            SortAnnotationsRet::Ok(p) => {
                if !p.is_null() {
                    let p = Box::from_raw(p);
                    cstr_free!(p.input);
                    cstr_free!(p.output);
                }
            }
            SortAnnotationsRet::Err(p) => {
                if !p.is_null() {
                    let p = Box::from_raw(p);
                    cstr_free!(p.message);
                }
            }
        }
    }

    /// Sorts the annotations in the given GTF or GFF3 file and writes the result to the output file.
    ///
    /// `result_ptr` is a pointer to a [SortAnnotationsRet] that will be set to the result of the operation.
    /// if you don't need the result, you can pass a null pointer.
    ///
    /// The return value is true if the operation was successful, false otherwise.
    ///
    /// # Safety
    /// input and output must be valid C strings that point to valid file paths.
    #[no_mangle]
    pub unsafe extern "C" fn gtfsort_sort_annotations(
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

    /// Sorts the annotations in the given GTF or GFF3 string and writes the result chunk by chunk to the output callback.
    ///
    /// The mode must be one of the following:
    /// - [GTFSORT_PARSE_MODE_GTF]
    /// - [GTFSORT_PARSE_MODE_GFF3]
    /// - [GTFSORT_PARSE_MODE_GFF]
    ///
    /// output is a callback function that will be called with the following arguments:
    /// - caller_data: a pointer to the caller data
    /// - output: a pointer to the output bytes
    /// - len: the length of the output bytes
    ///
    /// The callback function should return a null pointer in case of success, or an error message in case of failure.
    ///
    /// caller_data is a pointer to the caller data that will be passed to the output callback.
    ///
    /// result_ptr is a pointer to a SortAnnotationsRet that will be set to the result of the operation.
    /// if you don't need the result, you can pass a null pointer.
    ///
    /// the return value is true if the operation was successful, false otherwise.
    ///
    /// # Safety
    ///
    /// input must be a valid C string.
    ///
    /// The caller is responsible for freeing the error message in output callback.
    ///
    #[no_mangle]
    pub unsafe extern "C" fn gtfsort_sort_annotations_gtf_str(
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
