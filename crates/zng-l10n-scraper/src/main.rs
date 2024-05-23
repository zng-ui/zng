#![doc(html_favicon_url = "https://raw.githubusercontent.com/zng-ui/zng/master/examples/res/image/zng-logo-icon.png")]
#![doc(html_logo_url = "https://raw.githubusercontent.com/zng-ui/zng/master/examples/res/image/zng-logo.png")]
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]

fn main() {
    println!("zng-l10n-scraper is DEPRECATED\n");
    println!("Run these commands to upgrade:\n");
    println!("cargo uninstall zng-l10n-scraper");
    println!("cargo install cargo-zng");
    println!("cargo zng l10n --help");
}
