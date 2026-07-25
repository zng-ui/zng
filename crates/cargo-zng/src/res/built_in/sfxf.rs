use super::*;

const SFXF_HELP: &str = r#"
Build a self-extracting executable on the final pass

Apart from running on final this tool behaves exactly like .zr-sfx
"#;
pub(super) fn sfxf() {
    help(SFXF_HELP);
    if std::env::var(ZR_FINAL).is_ok() {
        sfx();
    } else {
        println!("zng-res::on-final=");
    }
}
