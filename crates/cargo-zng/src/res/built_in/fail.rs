use super::*;

const FAIL_HELP: &str = "
Print an error message and fail the build

The request file:
  some/dir/disallow.zr-fail.zr-rp
   | Don't copy ${ZR_REQUEST_DD} with a glob!

Prints an error message and fails the build if copied
";
pub(super) fn fail() {
    help(FAIL_HELP);
    let message = fs::read_to_string(ZR_REQUEST).unwrap_or_else(|e| fatal!("{e}"));
    fatal!("{message}");
}
