/// Converts a [`std::panic::catch_unwind`] payload to a str.
pub fn panic_str<'s>(payload: &'s Box<dyn std::any::Any + Send + 'static>) -> &'s str {
    try_panic_str(payload).unwrap_or("<unknown-panic-message-type>")
}

/// Converts a [`std::panic::catch_unwind`] payload to a str.
pub fn try_panic_str<'s>(payload: &'s Box<dyn std::any::Any + Send + 'static>) -> Option<&'s str> {
    if let Some(s) = payload.downcast_ref::<&str>() {
        Some(s)
    } else if let Some(s) = payload.downcast_ref::<String>() {
        Some(s)
    } else {
        None
    }
}

/// The result that is returned by [`std::panic::catch_unwind`].
pub type PanicResult<R> = std::thread::Result<R>; // TODO(breaking) replace this with Result<R, PanicError>
