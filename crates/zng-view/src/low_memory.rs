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
