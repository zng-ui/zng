fn main() {
    if let Some(v) = std::env::var("SFX_ARGS") {
        println!("SFX_ARGS:\n{v}");
        let sfx = v.lines().next().unwrap();
        let first = sfx_data(sfx, "first");
        let second = sfx_data(sfx, "second");
        println!("get-data:");
        println!("first = {first:?}");
        println!("second = {first:?}");
    }
}

fn sfx_data(sfx: &str, name: &str) -> String {
    let o = std::process::Command::new(sfx).env("SFX_GET_DATA", name).output().unwrap();
    assert!(o.status.success());
    str::from_utf8(&o.stdout).unwrap()
}
