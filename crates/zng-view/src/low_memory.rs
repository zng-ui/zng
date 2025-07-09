//! Low memory event for desktop systems

#[cfg(windows)]
mod windows {
    use windows::Win32::{
        Foundation::{BOOL, CloseHandle, HANDLE},
        System::Memory::*,
    };

    pub struct LowMemoryMonitor {
        handle: HANDLE,
        is_low: bool,
    }
    impl LowMemoryMonitor {
        pub fn new() -> Option<LowMemoryMonitor> {
            // SAFETY: its save, strongly typed call.
            let handle = match unsafe { CreateMemoryResourceNotification(LowMemoryResourceNotification) } {
                Ok(h) => h,
                Err(e) => {
                    tracing::error!("cannot create memory monitor, {e}");
                    return None;
                }
            };

            if handle.is_invalid() {
                tracing::error!("cannot create memory monitor, handle is invalid");
                return None;
            }

            Some(Self { handle, is_low: false })
        }

        pub fn notify(&mut self) -> bool {
            let mut is_low = BOOL::from(false);
            // SAFETY: strongly typed function called as documented in CreateMemoryResourceNotification msdn page.
            if let Err(e) = unsafe { QueryMemoryResourceNotification(self.handle, &mut is_low) } {
                tracing::error!("failed to query memory monitor, {e}");
                is_low = BOOL::from(false);
            }
            if self.is_low != is_low.as_bool() {
                self.is_low = is_low.as_bool();
                return self.is_low;
            }
            false
        }
    }
    impl Drop for LowMemoryMonitor {
        fn drop(&mut self) {
            // SAFETY: strongly typed function called as documented in CreateMemoryResourceNotification msdn page.
            if let Err(e) = unsafe { CloseHandle(self.handle) } {
                tracing::error!("failed to close memory monitor, {e}");
            }
        }
    }
}
#[cfg(windows)]
pub use windows::LowMemoryMonitor;

#[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
mod linux {
    use std::fs::File;
    use std::io::{BufRead, BufReader};

    pub struct LowMemoryMonitor {
        is_low: bool,
    }

    impl LowMemoryMonitor {
        pub fn new() -> Option<Self> {
            Some(Self { is_low: false })
        }

        pub fn notify(&mut self) -> bool {
            let meminfo = match File::open("/proc/meminfo") {
                Ok(f) => f,
                Err(e) => {
                    tracing::error!("cannot read /proc/meminfo, {e}");
                    return false;
                }
            };
            let reader = BufReader::new(meminfo);
            let mut available_kb = None;

            for line in reader.lines().map_while(Result::ok) {
                if line.starts_with("MemAvailable:") {
                    let parts: Vec<_> = line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        available_kb = parts[1].parse::<u64>().ok();
                        break;
                    }
                }
            }

            let available_kb = match available_kb {
                Some(kb) => kb,
                None => {
                    tracing::error!("cannot read MemAvailable from /proc/meminfo");
                    return false;
                }
            };
            let available_bytes = available_kb * 1024;
            let is_low = available_bytes < 200 * 1024 * 1024; // less than 200MB

            if self.is_low != is_low {
                self.is_low = is_low;
                return is_low;
            }

            false
        }
    }
}
#[cfg(any(
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd"
))]
pub use linux::LowMemoryMonitor;

#[cfg(target_os = "macos")]
mod macos {
    use libc::{_SC_PAGESIZE, HOST_VM_INFO64, KERN_SUCCESS, c_uint, host_statistics64, sysconf, vm_statistics64};
    use std::mem::MaybeUninit;
    #[allow(deprecated)] // suggestion says to use mach2, but that crate does not have this function
    fn mach_host_self() -> libc::mach_port_t {
        // SAFETY: this the correct usage
        unsafe { libc::mach_host_self() }
    }
    pub struct LowMemoryMonitor {
        is_low: bool,
        page_size: usize,
    }

    impl LowMemoryMonitor {
        pub fn new() -> Option<Self> {
            // SAFETY: this is the correct usage
            let page_size = unsafe { sysconf(_SC_PAGESIZE) };

            Some(Self {
                is_low: false,
                page_size: page_size as usize,
            })
        }

        pub fn notify(&mut self) -> bool {
            let mut vm_stats = MaybeUninit::<vm_statistics64>::uninit();

            let mut count = (std::mem::size_of::<vm_statistics64>() / std::mem::size_of::<u32>()) as c_uint;

            // SAFETY: this is the correct usage
            let result = unsafe { host_statistics64(mach_host_self(), HOST_VM_INFO64, vm_stats.as_mut_ptr() as *mut _, &mut count) };

            if result != KERN_SUCCESS {
                tracing::error!("host_statistics64 failed with code {result}");
                return false;
            }

            let stats = unsafe { vm_stats.assume_init() };

            // Inactive memory can be reclaimed by the OS, so it's also "available".
            let available_pages = stats.free_count + stats.inactive_count;
            let free_bytes = available_pages as u64 * self.page_size as u64;

            // less than 200MB
            let is_low = free_bytes < (200 * 1024 * 1024);

            if self.is_low != is_low {
                self.is_low = is_low;
                return is_low;
            }
            false
        }
    }
}
#[cfg(target_os = "macos")]
pub use macos::LowMemoryMonitor;

#[cfg(not(any(
    windows,
    target_os = "macos",
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    // target_os = "android", // winit provides LowMemory event for Android
)))]
#[non_exhaustive]
pub struct LowMemoryMonitor {}
#[cfg(not(any(
    windows,
    target_os = "macos",
    target_os = "linux",
    target_os = "dragonfly",
    target_os = "freebsd",
    target_os = "netbsd",
    target_os = "openbsd",
    // target_os = "android",
)))]
impl LowMemoryMonitor {
    pub fn new() -> Option<Self> {
        Some(Self {})
    }

    pub fn notify(&mut self) -> bool {
        false
    }
}
