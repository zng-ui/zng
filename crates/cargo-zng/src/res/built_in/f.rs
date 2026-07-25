use super::*;

const F_HELP: &str = r#"
Run the inner tool on final pass

The request file:
  source/warn.zr-warn.zr-f
   | Final Pass!

Will print the warning "Final Pass!" after all other requests are processed
"#;
pub(super) fn f() {
    help(F_HELP);
    if std::env::var(ZR_FINAL).is_ok() {
        let request = path(ZR_REQUEST);
        let target = path(ZR_TARGET);
        fs::copy(request, target).unwrap_or_else(|e| fatal!("{e}"));
    } else {
        println!("zng-res::on-final=");
    }
}
