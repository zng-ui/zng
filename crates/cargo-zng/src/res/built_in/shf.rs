use super::*;

const SHF_HELP: &str = r#"
Run a bash script on the final pass

Apart from running on final this tool behaves exactly like .zr-sh
"#;
pub(super) fn shf() {
    help(SHF_HELP);
    if std::env::var(ZR_FINAL).is_ok() {
        sh();
    } else {
        println!("zng-res::on-final=");
    }
}
