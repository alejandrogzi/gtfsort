use std::marker::PhantomData;

#[cfg(all(not(unix), not(windows)))]
compile_error!(
    "mmap is only supported on Unix and Windows platforms, please compile without the mmap feature"
);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Madvice {
    Normal,
    Random,
    Sequential,
    WillNeed,
    DontNeed,
    HugePage,
}

type CleanupFn<S> = Box<dyn FnOnce(&mut S)>;

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

    pub fn as_slice(&self) -> &[T] {
        unsafe { std::slice::from_raw_parts(self.ptr, self.size / std::mem::size_of::<T>()) }
    }

    pub fn madvise(&self, advice: &[Madvice]) -> Result<(), std::io::Error> {
        #[cfg(unix)]
        {
            let advice = advice.iter().fold(0, |acc, &a| {
                acc | match a {
                    Madvice::Normal => libc::MADV_NORMAL,
                    Madvice::Random => libc::MADV_RANDOM,
                    Madvice::Sequential => libc::MADV_SEQUENTIAL,
                    Madvice::WillNeed => libc::MADV_WILLNEED,
                    Madvice::DontNeed => libc::MADV_DONTNEED,
                    Madvice::HugePage => libc::MADV_HUGEPAGE,
                }
            });

            let ret = unsafe { libc::madvise(self.ptr as *mut _, self.size, advice) };

            if ret == -1 {
                return Err(std::io::Error::last_os_error());
            }
        }

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
                    log::warn!("munmap error: {}", std::io::Error::last_os_error());
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
    pub unsafe fn from_handle<F>(handle: &'a F, size: Option<usize>) -> Result<Self, std::io::Error>
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

        unsafe {
            let handle = CreateFileMappingW(
                HANDLE(handle.as_raw_handle()),
                None,
                PAGE_READONLY,
                size.unwrap_or(0) as u32,
                0,
                w!("GFF3_MMAP"),
            )?;

            if handle.0.is_null() {
                return Err(std::io::Error::last_os_error());
            }

            let ptr = MapViewOfFile(
                handle,
                FILE_MAP_READ | FILE_MAP_LARGE_PAGES,
                0,
                0,
                size.unwrap_or(0),
            );

            if ptr.Value.is_null() {
                return Err(std::io::Error::last_os_error());
            }

            let ptr_clone = ptr.clone();

            Ok(Self {
                ptr: ptr.Value as *const T,
                size: size.unwrap_or(0),
                cleanup: Some(Box::new(move |_this| {
                    UnmapViewOfFile(ptr_clone).ok();
                    CloseHandle(handle).ok();
                })),
                _marker: PhantomData,
            })
        }
    }
}

impl<T> Drop for MemoryMap<'_, T> {
    fn drop(&mut self) {
        if let Some(cleanup) = self.cleanup.take() {
            cleanup(self);
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
        unsafe { std::slice::from_raw_parts(self.ptr, self.size / std::mem::size_of::<T>()) }
    }

    pub fn as_mut_slice(&mut self) -> &mut [T] {
        unsafe { std::slice::from_raw_parts_mut(self.ptr, self.size / std::mem::size_of::<T>()) }
    }

    pub fn madvise(&self, advice: &[Madvice]) -> Result<(), std::io::Error> {
        #[cfg(unix)]
        {
            let advice = advice.iter().fold(0, |acc, &a| {
                acc | match a {
                    Madvice::Normal => libc::MADV_NORMAL,
                    Madvice::Random => libc::MADV_RANDOM,
                    Madvice::Sequential => libc::MADV_SEQUENTIAL,
                    Madvice::WillNeed => libc::MADV_WILLNEED,
                    Madvice::DontNeed => libc::MADV_DONTNEED,
                    Madvice::HugePage => libc::MADV_HUGEPAGE,
                }
            });

            let ret = unsafe { libc::madvise(self.ptr as *mut _, self.size, advice) };

            if ret == -1 {
                return Err(std::io::Error::last_os_error());
            }
        }

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
                    eprintln!("munmap failed: {}", std::io::Error::last_os_error());
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
    pub unsafe fn from_handle<F>(handle: &'a F, size: Option<usize>) -> Result<Self, std::io::Error>
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

        unsafe {
            let handle = CreateFileMappingW(
                HANDLE(handle.as_raw_handle()),
                None,
                PAGE_READWRITE,
                size.unwrap_or(0) as u32,
                0,
                w!("GFF3_MMAP_WRITE"),
            )?;

            if handle.0.is_null() {
                return Err(std::io::Error::last_os_error());
            }

            let ptr = MapViewOfFile(
                handle,
                FILE_MAP_WRITE | FILE_MAP_LARGE_PAGES,
                0,
                0,
                size.unwrap_or(0),
            );

            if ptr.Value.is_null() {
                return Err(std::io::Error::last_os_error());
            }

            let ptr_clone = ptr.clone();

            Ok(Self {
                ptr: ptr.Value as *mut T,
                size: size.unwrap_or(0),
                cleanup: Some(Box::new(move |_this| {
                    UnmapViewOfFile(ptr_clone).ok();
                    CloseHandle(handle).ok();
                })),
                _marker: PhantomData,
            })
        }
    }
}

impl<T> Drop for MemoryMapMut<'_, T> {
    fn drop(&mut self) {
        if let Some(cleanup) = self.cleanup.take() {
            cleanup(self);
        }
    }
}
