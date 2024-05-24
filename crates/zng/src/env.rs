//! Process external directories and files.
//!
//! This module contains functions to get external files associated with the installed app package. Note that
//! these values are associated with the executable, not just the `APP` context.
//!
//! ```
//! fn main() {
//!    // optional, but recommended package metadata init.
//!    zng::env::init("io.github.zng-ui", "Zng Developers", "Zng");
//!
//!    // get a path in the app config dir, the config dir is created if needed.    
//!    let my_config = zng::env::config("my-config.txt");
//!
//!    // read a config file, or create it
//!    if let Ok(c) = std::fs::read_to_string(&my_config) {
//!       println!("{c}");
//!    } else {
//!       std::fs::write(zng::env::config("my-config.txt"), b"Hello!").unwrap();
//!    }
//! }
//! ```
//!
//! The example above uses [`init`] to initialize the metadata used to find a good place for each directory, it then
//! uses [`config`] to write and read a file.

pub use zng_env::{app_unique_name, bin, cache, config, init, init_cache, init_config, init_res, res};
