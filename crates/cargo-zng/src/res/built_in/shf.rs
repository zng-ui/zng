use super::*;

// TODO(breaking) remove this

const SHF_HELP: &str = r#"
DEPRECATED

Use `.zr-sh.zr-f`
"#;
pub(super) fn shf() {
    help(SHF_HELP);
    if std::env::var(ZR_FINAL).is_ok() {
        warn!(".zr-shf is deprecated, use .zr-sh.zr-f");
        sh();
    } else {
        println!("zng-res::on-final=");
    }
}
