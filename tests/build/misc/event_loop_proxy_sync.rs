use zero_ui_core::app::{EventLoopProxy, EventLoopProxySync};

fn a(el: EventLoopProxy) {
    fn expected_error<T: Sync>(_: T) {}

    expected_error(el); // expect error here, otherwise we don't need the `EventLoopProxySync`.

    // if this fails check if `winit` implemented `Sync` to their event loop proxy,
    // in that case we can remove the Mutex based EventLoopProxySync.
}

fn b(el: EventLoopProxySync) {
    fn not_expected_error<T: Sync>(_: T) {}

    not_expected_error(el); // expect no error here.
}

fn main() {}
