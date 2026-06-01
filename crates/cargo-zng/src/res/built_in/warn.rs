use super::*;

const WARN_HELP: &str = "
Print a warning message

You can combine this with '.zr-rp' tool

The request file:
  source/warn.zr-warn.zr-rp
   | ${ZR_APP}!

Prints a warning with the value of ZR_APP
";
pub(super) fn warn() {
    help(WARN_HELP);
    let message = fs::read_to_string(path(ZR_REQUEST)).unwrap_or_else(|e| fatal!("{e}"));
    println!("zng-res::warning={message}");
}
