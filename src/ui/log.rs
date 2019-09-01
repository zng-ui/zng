use super::{LayoutSize, NextFrame, Ui, UiContainer};
use log::info;

pub trait UiLog: Ui + Sized {
    fn log_layout(self, target: &'static str) -> LogLayout<Self> {
        LogLayout::new(self, target)
    }

    fn log_render(self, target: &'static str) -> LogRender<Self> {
        LogRender::new(self, target)
    }
}
impl<T: Ui> UiLog for T {}

pub fn log_layout<T: Ui>(child: T, target: &'static str) -> LogLayout<T> {
    LogLayout::new(child, target)
}

pub fn log_render<T: Ui>(child: T, target: &'static str) -> LogRender<T> {
    LogRender::new(child, target)
}

pub struct LogLayout<T: Ui> {
    child: T,
    target: &'static str,
}

impl<T: Ui> LogLayout<T> {
    pub fn new(child: T, target: &'static str) -> Self {
        LogLayout { child, target }
    }
}

impl<T: Ui> UiContainer for LogLayout<T> {
    delegate_child!(child, T);

    fn measure(&mut self, available_size: LayoutSize) -> LayoutSize {
        let r = self.child.measure(available_size);
        info!(target: self.target, "measure({}) -> {}", available_size, r);
        r
    }

    fn arrange(&mut self, final_size: LayoutSize) {
        self.child.arrange(final_size);
        info!(target: self.target, "arrange({})", final_size);
    }
}
delegate_ui!(UiContainer, LogLayout<T>, T);

pub struct LogRender<T: Ui> {
    child: T,
    target: &'static str,
}

impl<T: Ui> LogRender<T> {
    pub fn new(child: T, target: &'static str) -> Self {
        LogRender { child, target }
    }
}

impl<T: Ui> UiContainer for LogRender<T> {
    delegate_child!(child, T);

    fn render(&self, f: &mut NextFrame) {
        self.child.render(f);
        info!(target: self.target, "render({})", f.final_size());
    }
}
delegate_ui!(UiContainer, LogRender<T>, T);

/// Log `"[{level}][{target}] {message}"` to stdout.
pub fn start_logger_for(target: &'static str) {
    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{level}][{target}] {message}",
                level = record.level(),
                target = record.target(),
                message = message,
            ))
        })
        .level(log::LevelFilter::Off)
        .level_for(target, log::LevelFilter::Trace)
        .chain(std::io::stdout())
        .apply()
        .ok();
}
