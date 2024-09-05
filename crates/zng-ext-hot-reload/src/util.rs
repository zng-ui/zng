use std::fmt;

use zng_txt::{ToTxt, Txt};

// format panic, code copied from `zng::crash_handler`
pub fn crash_handler(info: &std::panic::PanicHookInfo) {
    let backtrace = std::backtrace::Backtrace::capture();
    let panic = PanicInfo::from_hook(info);
    eprintln!("{panic}stack backtrace:\n{backtrace}");
}

#[derive(Debug)]
struct PanicInfo {
    pub thread: Txt,
    pub msg: Txt,
    pub file: Txt,
    pub line: u32,
    pub column: u32,
}
impl PanicInfo {
    pub fn from_hook(info: &std::panic::PanicHookInfo) -> Self {
        let current_thread = std::thread::current();
        let thread = current_thread.name().unwrap_or("<unnamed>");
        let msg = Self::payload(info.payload());

        let (file, line, column) = if let Some(l) = info.location() {
            (l.file(), l.line(), l.column())
        } else {
            ("<unknown>", 0, 0)
        };
        Self {
            thread: thread.to_txt(),
            msg,
            file: file.to_txt(),
            line,
            column,
        }
    }

    fn payload(p: &dyn std::any::Any) -> Txt {
        match p.downcast_ref::<&'static str>() {
            Some(s) => s,
            None => match p.downcast_ref::<String>() {
                Some(s) => &s[..],
                None => "Box<dyn Any>",
            },
        }
        .to_txt()
    }
}
impl std::error::Error for PanicInfo {}
impl fmt::Display for PanicInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "thread '{}' panicked at {}:{}:{}:",
            self.thread, self.file, self.line, self.column
        )?;
        for line in self.msg.lines() {
            writeln!(f, "   {line}")?;
        }
        Ok(())
    }
}
