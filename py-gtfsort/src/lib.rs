use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use pyo3::wrap_pyfunction;

use num_cpus;

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use gtfsort::{sort_annotations, sort_annotations_string};

#[pyfunction]
fn sort(py: Python, input: PyObject, output: PyObject, threads: Option<usize>) -> PyResult<String> {
    let input = PathBuf::from(input.extract::<String>(py)?);
    let output = PathBuf::from(output.extract::<String>(py)?);

    let job_info = sort_annotations(&input, &output, threads.unwrap_or(num_cpus::get()));

    match job_info {
        Ok(_) => Ok(format!(
            "File succesfully sorted! Result at {}. Used {} Mb. Elapsed: {}",
            &output.to_string_lossy(),
            job_info.as_ref().unwrap().end_mem_mb.unwrap_or(f64::NAN)
                - job_info.as_ref().unwrap().start_mem_mb.unwrap_or(f64::NAN),
            job_info.as_ref().unwrap().parsing_secs
                + job_info.as_ref().unwrap().indexing_secs
                + job_info.as_ref().unwrap().writing_secs
        )),
        Err(e) => Ok(format!("Error: {}", e)),
    }
}

#[pyfunction]
fn sort_from_string<'a>(
    py: Python,
    input: &str,
    output_callback: PyObject,
    mut threads: usize,
) -> PyResult<()> {
    if threads == 0 {
        threads = num_cpus::get();
    }

    let output_data = Arc::new(Mutex::new(Vec::new()));

    let mut output_callback_rust = {
        let output_data = Arc::clone(&output_data);
        move |data: &[u8]| -> std::io::Result<usize> {
            let mut output = output_data.lock().unwrap();
            output.extend_from_slice(data);
            Ok(data.len())
        }
    };

    match sort_annotations_string::<b' ', _>(input, &mut output_callback_rust, threads) {
        Ok(_) => {
            let output = output_data.lock().unwrap();
            let py_bytes = PyBytes::new(py, &output);
            output_callback.call1(py, (py_bytes,))?;
            Ok(())
        }
        Err(e) => Err(PyValueError::new_err(format!("Error: {:?}", e))),
    }
}

#[pymodule]
#[pyo3(name = "gxfsort")]
fn py_gtfsort(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(sort, m)?)?;
    m.add_function(wrap_pyfunction!(sort_from_string, m)?)?;
    Ok(())
}
