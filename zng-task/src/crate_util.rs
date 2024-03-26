/// Converts a [`std::panic::catch_unwind`] payload to a str.
pub fn panic_str<'s>(payload: &'s Box<dyn std::any::Any + Send + 'static>) -> &'s str {
    if let Some(s) = payload.downcast_ref::<&str>() {
        s
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s
    } else {
        "<unknown-panic-message-type>"
    }
}

/// The result that is returned by [`std::panic::catch_unwind`].
pub type PanicResult<R> = std::thread::Result<R>;
