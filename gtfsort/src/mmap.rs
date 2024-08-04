use std::marker::PhantomData;

#[cfg(all(not(unix), not(windows)))]
compile_error!(
    "mmap is only supported on Unix and Windows platforms, please compile without the mmap feature"
);

#[cfg(windows)]
macro_rules! high32 {
    ($x:expr) => {
        ($x >> 32) as u32
    };
}

#[cfg(windows)]
macro_rules! low32 {
    ($x:expr) => {
        $x as u32
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Madvice {
    Normal,
    Random,
    Sequential,
    WillNeed,
    DontNeed,
    HugePage,
}

type CleanupFn<S> = Box<dyn FnOnce(&mut S) -> std::io::Result<()>>;

pub struct MemoryMap<'a, T> {
    ptr: *const T,
    size: usize,
    cleanup: Option<CleanupFn<Self>>,
    _marker: PhantomData<&'a T>,
}

impl<'a, T> MemoryMap<'a, T> {
    /// Creates a new MemoryMap instance from a pointer and size.
    ///
    /// # Safety
    /// ptr must be a valid pointer to a memory-mapped region of size bytes.
    pub unsafe fn new(ptr: *const T, size: usize) -> Self {
        Self {
            ptr,
            size,
            cleanup: None,
            _marker: PhantomData,
        }
    }

    pub fn size_bytes(&self) -> usize {
        self.size
    }

    pub fn as_slice(&self) -> &[T] {
        if self.size == 0 {
            return &[];
        }
        unsafe { std::slice::from_raw_parts(self.ptr, self.size / std::mem::size_of::<T>()) }
    }

    #[cfg(unix)]
    pub fn madvise(&self, advice: &[Madvice]) -> Result<(), std::io::Error> {
        if self.ptr.is_null() {
            return Ok(());
        }

        #[allow(unreachable_patterns)]
        let advice = advice.iter().fold(0, |acc, &a| {
            acc | match a {
                Madvice::Normal => libc::MADV_NORMAL,
                Madvice::Random => libc::MADV_RANDOM,
                Madvice::Sequential => libc::MADV_SEQUENTIAL,
                Madvice::WillNeed => libc::MADV_WILLNEED,
                Madvice::DontNeed => libc::MADV_DONTNEED,
                #[cfg(target_os = "linux")]
                Madvice::HugePage => libc::MADV_HUGEPAGE,
                _ => 0,
            }
        });

        let ret = unsafe { libc::madvise(self.ptr as *mut _, self.size, advice) };

        if ret == -1 {
            return Err(std::io::Error::last_os_error());
        }

        Ok(())
    }

    #[cfg(not(unix))]
    pub fn madvise(&self, _advice: &[Madvice]) -> Result<(), std::io::Error> {
        Ok(())
    }

    #[cfg(unix)]
    /// Creates a new MemoryMap instance from a file descriptor and size.
    ///
    /// # Safety
    /// fd must be a valid file descriptor.
    /// The file descriptor must be open and readable.
    /// Size must be a valid size for the file descriptor.
    pub unsafe fn from_file<F>(fd: &'a F, size: usize) -> Result<Self, std::io::Error>
    where
        F: std::os::unix::io::AsRawFd,
    {
        if size == 0 {
            return Ok(Self {
                ptr: std::ptr::null(),
                size,
                cleanup: None,
                _marker: PhantomData,
            });
        }

        let ptr = libc::mmap(
            std::ptr::null_mut(),
            size,
            libc::PROT_READ,
            libc::MAP_SHARED,
            fd.as_raw_fd(),
            0,
        );

        if ptr == libc::MAP_FAILED {
            return Err(std::io::Error::last_os_error());
        }

        Ok(Self {
            ptr: ptr as *const T,
            size,
            cleanup: Some(Box::new(move |this| unsafe {
                let ret = libc::munmap(this.ptr as *mut _, this.size);
                if ret == -1 {
                    let e = std::io::Error::last_os_error();
                    log::warn!("munmap error: {}", e);
                    Err(e)
                } else {
                    Ok(())
                }
            })),
            _marker: PhantomData,
        })
    }

    #[cfg(windows)]
    /// Creates a new MemoryMap instance from a file handle and size.
    ///
    /// # Safety
    /// handle must be a valid file handle.
    /// The file handle must be open and readable.
    /// Size must be a valid size for the file handle.
    pub unsafe fn from_handle<F>(handle: &'a F, size: usize) -> Result<Self, std::io::Error>
    where
        F: std::os::windows::io::AsRawHandle,
    {
        use windows::{
            core::*,
            Win32::{
                Foundation::{CloseHandle, HANDLE},
                System::Memory::*,
            },
        };

        if size == 0 {
            return Ok(Self {
                ptr: std::ptr::null(),
                size,
                cleanup: None,
                _marker: PhantomData,
            });
        }

        unsafe {
            let handle = CreateFileMappingW(
                HANDLE(handle.as_raw_handle()),
                None,
                PAGE_READONLY,
                high32!(size),
                low32!(size),
                PCWSTR::null(),
            )?;

            if handle.0.is_null() {
                return Err(std::io::Error::last_os_error());
            }

            let ptr = MapViewOfFile(handle, FILE_MAP_READ, 0, 0, size);

            if ptr.Value.is_null() {
                return Err(std::io::Error::last_os_error());
            }

            Ok(Self {
                ptr: ptr.Value as *const T,
                size,
                cleanup: Some(Box::new(move |_this| {
                    UnmapViewOfFile(ptr)?;
                    CloseHandle(handle)?;
                    Ok(())
                })),
                _marker: PhantomData,
            })
        }
    }

    pub fn close(mut self) -> Result<(), std::io::Error> {
        if let Some(cleanup) = self.cleanup.take() {
            cleanup(&mut self)?;
        }
        Ok(())
    }
}

impl<T> Drop for MemoryMap<'_, T> {
    fn drop(&mut self) {
        if let Some(cleanup) = self.cleanup.take() {
            cleanup(self).expect("failed to unmap memory, and error was ignored");
        }
    }
}

pub struct MemoryMapMut<'a, T> {
    ptr: *mut T,
    size: usize,
    cleanup: Option<CleanupFn<Self>>,
    _marker: PhantomData<&'a mut T>,
}

impl<'a, T> MemoryMapMut<'a, T> {
    /// Creates a new MemoryMapMut instance from a pointer and size.
    ///
    /// # Safety
    /// ptr must be a valid pointer to a memory-mapped region of size bytes.
    pub unsafe fn new(ptr: *mut T, size: usize) -> Self {
        Self {
            ptr,
            size,
            cleanup: None,
            _marker: PhantomData,
        }
    }

    pub fn as_slice(&self) -> &[T] {
        if self.size == 0 {
            return &[];
        }
        unsafe { std::slice::from_raw_parts(self.ptr, self.size / std::mem::size_of::<T>()) }
    }

    pub fn as_mut_slice(&mut self) -> &mut [T] {
        if self.size == 0 {
            return &mut [];
        }
        unsafe { std::slice::from_raw_parts_mut(self.ptr, self.size / std::mem::size_of::<T>()) }
    }

    #[cfg(unix)]
    pub fn madvise(&self, advice: &[Madvice]) -> Result<(), std::io::Error> {
        if self.ptr.is_null() {
            return Ok(());
        }
        #[allow(unreachable_patterns)]
        let advice = advice.iter().fold(0, |acc, &a| {
            acc | match a {
                Madvice::Normal => libc::MADV_NORMAL,
                Madvice::Random => libc::MADV_RANDOM,
                Madvice::Sequential => libc::MADV_SEQUENTIAL,
                Madvice::WillNeed => libc::MADV_WILLNEED,
                Madvice::DontNeed => libc::MADV_DONTNEED,
                #[cfg(target_os = "linux")]
                Madvice::HugePage => libc::MADV_HUGEPAGE,
                _ => 0,
            }
        });

        let ret = unsafe { libc::madvise(self.ptr as *mut _, self.size, advice) };

        if ret == -1 {
            return Err(std::io::Error::last_os_error());
        }

        Ok(())
    }

    #[cfg(not(unix))]
    pub fn madvise(&self, _advice: &[Madvice]) -> Result<(), std::io::Error> {
        Ok(())
    }

    #[cfg(unix)]
    /// Creates a new MemoryMapMut instance from a file descriptor and size.
    ///
    /// # Safety
    /// fd must be a valid file descriptor.
    /// The file descriptor must be open and readable.
    /// Size must be a valid size for the file descriptor.
    pub unsafe fn from_file<F>(fd: &'a F, size: usize) -> Result<Self, std::io::Error>
    where
        F: std::os::unix::io::AsRawFd,
    {
        if size == 0 {
            return Ok(Self {
                ptr: std::ptr::null_mut(),
                size,
                cleanup: None,
                _marker: PhantomData,
            });
        }

        let ptr = libc::mmap(
            std::ptr::null_mut(),
            size,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_SHARED,
            fd.as_raw_fd(),
            0,
        );

        if ptr == libc::MAP_FAILED {
            return Err(std::io::Error::last_os_error());
        }

        Ok(Self {
            ptr: ptr as *mut T,
            size,
            cleanup: Some(Box::new(move |this| unsafe {
                let ret = libc::munmap(this.ptr as *mut _, this.size);
                if ret == -1 {
                    let e = std::io::Error::last_os_error();
                    eprintln!("munmap failed: {}", e);
                    Err(e)
                } else {
                    Ok(())
                }
            })),
            _marker: PhantomData,
        })
    }

    #[cfg(windows)]
    /// Creates a new MemoryMapMut instance from a file handle and size.
    /// # Safety
    /// handle must be a valid file handle.
    /// The file handle must be open and writable.
    /// Size must be a valid size for the file handle.
    pub unsafe fn from_handle<F>(handle: &'a F, size: usize) -> Result<Self, std::io::Error>
    where
        F: std::os::windows::io::AsRawHandle,
    {
        use windows::{
            core::*,
            Win32::{
                Foundation::{CloseHandle, HANDLE},
                System::Memory::*,
            },
        };

        if size == 0 {
            return Ok(Self {
                ptr: std::ptr::null_mut(),
                size,
                cleanup: None,
                _marker: PhantomData,
            });
        }

        unsafe {
            let handle = CreateFileMappingW(
                HANDLE(handle.as_raw_handle()),
                None,
                PAGE_READWRITE,
                high32!(size),
                low32!(size),
                PCWSTR::null(),
            )?;

            if handle.0.is_null() {
                return Err(std::io::Error::last_os_error());
            }

            let ptr = MapViewOfFile(handle, FILE_MAP_WRITE, 0, 0, size);

            if ptr.Value.is_null() {
                return Err(std::io::Error::last_os_error());
            }

            Ok(Self {
                ptr: ptr.Value as *mut T,
                size,
                cleanup: Some(Box::new(move |_this| {
                    UnmapViewOfFile(ptr)?;
                    CloseHandle(handle)?;
                    Ok(())
                })),
                _marker: PhantomData,
            })
        }
    }

    pub fn close(mut self) -> Result<(), std::io::Error> {
        if let Some(cleanup) = self.cleanup.take() {
            cleanup(&mut self)?;
        }
        Ok(())
    }
}

impl<T> Drop for MemoryMapMut<'_, T> {
    fn drop(&mut self) {
        if let Some(cleanup) = self.cleanup.take() {
            cleanup(self).expect("failed to unmap memory, and error was ignored");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs::OpenOptions, io::Write, path::PathBuf, sync::atomic::AtomicU64};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    fn tempfile_ro(data: &[u8]) -> (PathBuf, std::fs::File) {
        let path = std::env::temp_dir().join(format!(
            "gtfsort_mmap_test_{}",
            COUNTER.fetch_add(1, std::sync::atomic::Ordering::AcqRel)
        ));

        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path)
            .unwrap();

        file.write_all(data).unwrap();

        file.flush().unwrap();
        drop(file);

        (
            path.clone(),
            OpenOptions::new().read(true).open(&path).unwrap(),
        )
    }

    fn tempfile_rw(data: &[u8]) -> (PathBuf, std::fs::File) {
        let path = std::env::temp_dir().join(format!(
            "gtfsort_mmap_test_{}",
            COUNTER.fetch_add(1, std::sync::atomic::Ordering::AcqRel)
        ));

        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path)
            .unwrap();

        file.write_all(data).unwrap();

        file.flush().unwrap();

        (path, file)
    }

    #[test]
    fn test_mmap() {
        let (path, file) = tempfile_ro(b"hello world");

        #[cfg(unix)]
        let mmap = unsafe { MemoryMap::<u8>::from_file(&file, 5).unwrap() };
        #[cfg(windows)]
        let mmap = unsafe { MemoryMap::<u8>::from_handle(&file, 5).unwrap() };

        assert_eq!(mmap.as_slice(), b"hello");

        mmap.close().unwrap();

        drop(file);

        assert_eq!(std::fs::read_to_string(path).unwrap(), "hello world");
    }

    #[test]
    fn test_mmap_mut() {
        let (path, file) = tempfile_rw(b"hello world");

        #[cfg(unix)]
        let mut mmap = unsafe { MemoryMapMut::<u8>::from_file(&file, 11).unwrap() };
        #[cfg(windows)]
        let mut mmap = unsafe { MemoryMapMut::<u8>::from_handle(&file, 11).unwrap() };

        assert_eq!(mmap.as_slice(), b"hello world");

        mmap.as_mut_slice()["hello ".len()..].copy_from_slice(b"WORLD");

        assert_eq!(mmap.as_slice(), b"hello WORLD");

        mmap.close().unwrap();

        drop(file);

        assert_eq!(std::fs::read_to_string(path).unwrap(), "hello WORLD");
    }

    #[test]
    fn test_mmap_zero_size() {
        let (path, file) = tempfile_ro(b"");

        #[cfg(unix)]
        let mmap = unsafe { MemoryMap::<u8>::from_file(&file, 0).unwrap() };
        #[cfg(windows)]
        let mmap = unsafe { MemoryMap::<u8>::from_handle(&file, 0).unwrap() };

        assert_eq!(mmap.as_slice(), b"");

        mmap.close().unwrap();

        drop(file);

        assert_eq!(std::fs::read_to_string(path).unwrap(), "");
    }

    #[test]
    fn test_mmap_mut_zero_size() {
        let (path, file) = tempfile_rw(b"");

        #[cfg(unix)]
        let mmap = unsafe { MemoryMapMut::<u8>::from_file(&file, 0).unwrap() };
        #[cfg(windows)]
        let mmap = unsafe { MemoryMapMut::<u8>::from_handle(&file, 0).unwrap() };

        assert_eq!(mmap.as_slice(), b"");

        mmap.close().unwrap();

        drop(file);

        assert_eq!(std::fs::read_to_string(path).unwrap(), "");
    }

    #[test]
    fn test_mmap_madvise() {
        let (path, file) = tempfile_ro(b"hello world");

        #[cfg(unix)]
        let mmap = unsafe { MemoryMap::<u8>::from_file(&file, 5).unwrap() };
        #[cfg(windows)]
        let mmap = unsafe { MemoryMap::<u8>::from_handle(&file, 5).unwrap() };

        mmap.madvise(&[Madvice::Random]).unwrap();

        mmap.close().unwrap();

        drop(file);

        assert_eq!(std::fs::read_to_string(path).unwrap(), "hello world");
    }
}
