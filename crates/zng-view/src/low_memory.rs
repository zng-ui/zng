//! Low memory event for desktop systems

#[cfg(windows)]
mod windows {
    pub struct MemoryPressureCheck {
        // handle: HANDLE,
        is_low: bool,
    }
    impl Check {
        pub fn new() -> Self {
            // !!: TODO, use https://learn.microsoft.com/en-us/windows/win32/api/memoryapi/nf-memoryapi-creatememoryresourcenotification
            Self { is_low: false }
        }

        pub fn notify(&mut self) -> bool {
            // !!: TODO, use https://learn.microsoft.com/en-us/windows/win32/api/memoryapi/nf-memoryapi-querymemoryresourcenotification
            // !!: TODO, return true if self.is_low changes
            false
        }
    }
}

#[cfg(windows)]
pub use windows::MemoryPressureCheck;
