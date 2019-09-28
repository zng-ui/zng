use super::{impl_ui_crate, LayoutSize, NextFrame, Ui};
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

#[derive(new)]
pub struct LogLayout<T: Ui> {
    child: T,
    target: &'static str,
}

#[impl_ui_crate(child)]
impl<T: Ui> LogLayout<T> {
    #[Ui]
    fn measure(&mut self, available_size: LayoutSize) -> LayoutSize {
        let r = self.child.measure(available_size);
        info!(target: self.target, "measure({}) -> {}", available_size, r);
        r
    }

    #[Ui]
    fn arrange(&mut self, final_size: LayoutSize) {
        self.child.arrange(final_size);
        info!(target: self.target, "arrange({})", final_size);
    }
}

#[derive(new)]
pub struct LogRender<T: Ui> {
    child: T,
    target: &'static str,
}

#[impl_ui_crate(child)]
impl<T: Ui> LogRender<T> {
    #[Ui]
    fn render(&self, f: &mut NextFrame) {
        self.child.render(f);
        info!(target: self.target, "render({})", f.final_size());
    }
}

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
