#![allow(dead_code)]

use std::{
    collections::VecDeque,
    io::{BufRead, BufReader, Read},
    ops::Deref,
    path::{Path, PathBuf},
    sync::Once,
};

use flate2::read::GzDecoder;
use log::Level;

// https://stackoverflow.com/a/40234666/9739737
#[macro_export]
macro_rules! current_func {
    () => {{
        fn f() {}
        fn type_name_of<T>(_: T) -> &'static str {
            std::any::type_name::<T>()
        }
        let name = type_name_of(f);
        name.strip_suffix("::f").unwrap()
    }};
}

pub struct TempFile {
    path: PathBuf,
    cleanup: bool,
}

impl TempFile {
    pub fn new(name: &str, cleanup: bool) -> Self {
        let path = std::env::temp_dir().join(name);
        Self { path, cleanup }
    }
}

impl Deref for TempFile {
    type Target = PathBuf;

    fn deref(&self) -> &Self::Target {
        &self.path
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        if self.cleanup {
            std::fs::remove_file(&self.path).unwrap();
        }
    }
}

pub struct OnlyChromosomes<R> {
    inner: BufReader<R>,
    buf: Option<VecDeque<u8>>,
    chrom: &'static [&'static str],
}

impl<R: Read> OnlyChromosomes<R> {
    pub fn new(inner: R, chrom: &'static [&'static str]) -> Self {
        Self {
            inner: BufReader::new(inner),
            buf: None,
            chrom,
        }
    }
    pub fn buffer_more(&mut self) -> std::io::Result<usize> {
        let mut tot = 0;
        loop {
            let mut line = Vec::new();
            let n = self.inner.read_until(b'\n', &mut line)?;
            if n == 0 {
                return Ok(0);
            }
            tot += n;

            if line.starts_with(b"#")
                || self
                    .chrom
                    .iter()
                    .any(|&c| line.split(|c| *c == b'\t').next() == Some(c.as_bytes()))
            {
                let line = line.into_iter().collect::<VecDeque<_>>();
                self.buf = Some(line);
                return Ok(tot);
            }
        }
    }
}

impl<R: Read> Read for OnlyChromosomes<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }

        if self.buf.is_none() {
            self.buffer_more()?;
        }

        let mut n = 0;
        while let Some(byte) = self.buf.as_mut().unwrap().pop_front() {
            buf[n] = byte;
            n += 1;
            if n == buf.len() {
                break;
            }
        }

        if n == 0 {
            if self.buffer_more()? == 0 {
                Ok(0)
            } else {
                self.read(buf)
            }
        } else {
            Ok(n)
        }
    }
}

pub const TEST_FILE_GFF3_GENCODE_MOUSE_M35_FILENAME: &str =
    "gencode.vM35.chr_patch_hapl_scaff.basic.annotation.gff3";
pub const TEST_FILE_GFF3_GENCODE_MOUSE_M35_URL: &str  = "https://ftp.ebi.ac.uk/pub/databases/gencode/Gencode_mouse/release_M35/gencode.vM35.chr_patch_hapl_scaff.basic.annotation.gff3.gz";
static TEST_FILE_GFF3_GENCODE_MOUSE_M35_CELL: Once = Once::new();
static mut TEST_FILE_GFF3_GENCODE_MOUSE_M35: Option<TestFile> = None;
pub const TEST_FILE_GFF3_GENCODE_MOUSE_M35_TRANSFORMER: &dyn Fn(Box<dyn Read>) -> Box<dyn Read> =
    &|r| {
        Box::new(OnlyChromosomes::new(
            GzDecoder::new(r),
            &[
                "chr1",
                "chr2",
                "chr3",
                "chr5",
                "GL456221.1",
                "chrM",
                "ch11",
                "ch17",
            ],
        ))
    };
pub const TEST_FILE_GFF3_GENCODE_MOUSE_M35_EXPECT_OUTPUT_CKSUM: [&str; 1] = ["f6f3eb1d"];
pub fn get_test_file_gff3_gencode_mouse_m35() -> &'static TestFile {
    TEST_FILE_GFF3_GENCODE_MOUSE_M35_CELL.call_once(|| unsafe {
        TEST_FILE_GFF3_GENCODE_MOUSE_M35 = Some(TestFile::from_url(
            TEST_FILE_GFF3_GENCODE_MOUSE_M35_FILENAME,
            TEST_FILE_GFF3_GENCODE_MOUSE_M35_URL,
            &TEST_FILE_GFF3_GENCODE_MOUSE_M35_TRANSFORMER,
            &TEST_FILE_GFF3_GENCODE_MOUSE_M35_EXPECT_OUTPUT_CKSUM,
        ));
    });

    unsafe { TEST_FILE_GFF3_GENCODE_MOUSE_M35.as_ref().unwrap() }
}

pub fn crc32_hex<R: Read>(mut r: R) -> String {
    use crc::{Crc, CRC_32_CKSUM};

    let crc = Crc::<u32>::new(&CRC_32_CKSUM);
    let mut digest = crc.digest();

    let mut buffer = [0; 1024];

    loop {
        let n = r.read(&mut buffer).unwrap();
        if n == 0 {
            break;
        }
        digest.update(&buffer[..n]);
    }

    format!("{:08x}", digest.finalize())
}

pub struct TestFile {
    name: String,
    expect_output_cksum: Vec<&'static str>,
}

impl TestFile {
    pub fn new_fs(name: &str, expect_output_cksum: &[&'static str]) -> Self {
        Self {
            name: name.to_string(),
            expect_output_cksum: expect_output_cksum.to_vec(),
        }
    }
    pub fn from_url<RO: Read + ?Sized, F: Fn(Box<dyn Read>) -> Box<RO>>(
        cache_name: &str,
        url: &str,
        pipe: &F,
        expect_output_cksum: &[&'static str],
    ) -> Self {
        let tmpdir = std::env::temp_dir();

        let name = tmpdir.join(cache_name).to_string_lossy().to_string();
        let path = Path::new(&name);

        if path.exists() {
            return Self {
                name: path.to_str().unwrap().to_string(),
                expect_output_cksum: expect_output_cksum.to_vec(),
            };
        }

        let mut file = std::fs::File::create(path).unwrap();

        let resp = reqwest::blocking::get(url).unwrap();

        std::io::copy(&mut pipe(Box::new(resp)), &mut file).unwrap();

        Self::new_fs(name.as_str(), expect_output_cksum)
    }
    pub fn execute_test<F: FnOnce(&str) -> String>(&self, name: &str, f: F) {
        let output_cksum = f(&self.name);

        if self.expect_output_cksum.is_empty() {
            eprintln!("{}: not comparing cksum, got: {}", name, output_cksum);
        } else {
            assert!(
                self.expect_output_cksum.contains(&output_cksum.as_str()),
                "{}: cksum mismatch, got: {}",
                name,
                output_cksum
            );
        }
    }
}

static TEST_LOGGER_INIT: Once = Once::new();

pub fn ensure_logger_initialized() {
    TEST_LOGGER_INIT.call_once(|| {
        simple_logger::init_with_level(Level::Info).unwrap();
    });
}
