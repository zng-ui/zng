#![doc(html_favicon_url = "https://zng-ui.github.io/res/zng-logo-icon.png")]
#![doc(html_logo_url = "https://zng-ui.github.io/res/zng-logo.png")]
//!
//! Proc-macros for `zng-env`.
//!
//! # Crate
//!
#![doc = include_str!(concat!("../", std::env!("CARGO_PKG_README")))]
#![warn(unused_extern_crates)]
#![warn(missing_docs)]

use std::path::PathBuf;

use proc_macro::TokenStream;
use semver::Version;

#[macro_use]
extern crate quote;

#[doc(hidden)]
#[proc_macro]
pub fn init_parse(crate_: TokenStream) -> TokenStream {
    let crate_ = proc_macro2::TokenStream::from(crate_);

    let manifest = match std::env::var("CARGO_MANIFEST_DIR") {
        Ok(m) => PathBuf::from(m).join("Cargo.toml"),
        Err(e) => {
            let msg = format!("missing CARGO_MANIFEST_DIR, {e}");
            return quote! {
                compile_error!(#msg)
            }
            .into();
        }
    };
    let manifest_str = match std::fs::read_to_string(&manifest) {
        Ok(s) => s,
        Err(e) => {
            let msg = format!("cannot read `{}`, {e}", manifest.display());
            return quote! {
                compile_error!(#msg)
            }
            .into();
        }
    };

    let m: Manifest = match toml::from_str(&manifest_str) {
        Ok(m) => m,
        Err(e) => {
            let msg = format!("cannot parse Cargo.toml manifest, {e}");
            return quote! {
                compile_error!(#msg)
            }
            .into();
        }
    };

    let pkg_name = m.package.name;
    let pkg_authors = m.package.authors.unwrap_or_default();
    let (major, minor, patch, pre, build) = {
        let p = m.package.version;
        (p.major, p.minor, p.patch, p.pre.to_string(), p.build.to_string())
    };
    let description = m.package.description.unwrap_or_default();
    let homepage = m.package.homepage.unwrap_or_default();
    let license = m.package.license.unwrap_or_default();
    let mut app = "";
    let mut org = "";
    let mut app_id = String::new();
    let mut has_about = false;
    let mut meta_keys = vec![];
    let mut meta_values = vec![];
    if let Some(zng) = m.package.metadata.as_ref().and_then(|m| m.zng.as_ref())
        && !zng.about.is_empty()
    {
        let s = |key: &str| match zng.about.get(key) {
            Some(toml::Value::String(s)) => s.as_str(),
            _ => "",
        };
        has_about = true;
        app = s("app");
        org = s("org");
        app_id = clean_id(s("app_id"));
        for (k, v) in &zng.about {
            if let toml::Value::String(v) = v
                && !["app", "org", "app_id"].contains(&k.as_str())
            {
                meta_keys.push(k);
                meta_values.push(v);
            }
        }
    }
    if app.is_empty() {
        app = &pkg_name;
    }
    if org.is_empty() {
        org = pkg_authors.first().map(|s| s.as_str()).unwrap_or_default();
    }
    if app_id.is_empty() {
        let qualifier = meta_keys
            .iter()
            .position(|k| k.as_str() == "qualifier")
            .map(|i| meta_values[i].as_str())
            .unwrap_or_default();
        app_id = clean_id(&format!("{}.{}.{}", qualifier, org, app));
    }

    /*
    pub fn macro_new(
        pkg_name: &'static str,
        pkg_authors: &[&'static str],
        (major, minor, patch, pre, build): (u64, u64, u64, &'static str, &'static str),
        app_id: &'static str,
        app: &'static str,
        org: &'static str,
        description: &'static str,
        homepage: &'static str,
        license: &'static str,
        has_about: bool,
        meta: &[(&'static str, &'static str)],
    )
     */
    quote! {
        #crate_::init(#crate_::About::macro_new(
            #pkg_name,
            &[#(#pkg_authors),*],
            (#major, #minor, #patch, #pre, #build),
            #app_id,
            #app,
            #org,
            #description,
            #homepage,
            #license,
            #has_about,
            &[#( (#meta_keys, #meta_values) ),*],
        ))
    }
    .into()
}

/// * At least one identifier, dot-separated.
/// * Each identifier must contain ASCII letters, ASCII digits and underscore only.
/// * Each identifier must start with a letter.
/// * All lowercase.
fn clean_id(raw: &str) -> String {
    let mut r = String::new();
    let mut sep = "";
    for i in raw.split('.') {
        let i = i.trim();
        if i.is_empty() {
            continue;
        }
        r.push_str(sep);
        for (i, c) in i.trim().char_indices() {
            if i == 0 {
                if !c.is_ascii_alphabetic() {
                    r.push('i');
                } else {
                    r.push(c.to_ascii_lowercase());
                }
            } else if c.is_ascii_alphanumeric() || c == '_' {
                r.push(c.to_ascii_lowercase());
            } else {
                r.push('_');
            }
        }
        sep = ".";
    }
    r
}

#[derive(serde::Deserialize)]
struct Manifest {
    package: Package,
}
#[derive(serde::Deserialize)]
struct Package {
    name: String,
    version: Version,
    description: Option<String>,
    homepage: Option<String>,
    license: Option<String>,
    authors: Option<Box<[String]>>,
    metadata: Option<Metadata>,
}
#[derive(serde::Deserialize)]
struct Metadata {
    zng: Option<Zng>,
}
#[derive(serde::Deserialize)]
struct Zng {
    about: toml::Table,
}

#[doc(hidden)]
#[proc_macro]
// #[cfg(target_arch = "wasm32")] // cannot do this, target_arch is the build system arch
pub fn wasm_process_start(crate_closure: TokenStream) -> TokenStream {
    use quote::TokenStreamExt as _;

    let crate_closure = proc_macro2::TokenStream::from(crate_closure);
    let mut crate_ = proc_macro2::TokenStream::new();
    let mut crate_ok = false;
    let mut closure = proc_macro2::TokenStream::new();

    for tt in crate_closure {
        if crate_ok {
            closure.append(tt);
        } else if matches!(&tt, proc_macro2::TokenTree::Punct(p) if p.as_char() == ',') {
            crate_ok = true;
        } else {
            crate_.append(tt);
        }
    }

    use sha2::Digest;
    let mut start_ident = sha2::Sha256::new();
    start_ident.update(closure.to_string().as_bytes());
    let start_ident = format!("__zng_env_start_{:x}", start_ident.finalize());
    let start_ident = proc_macro2::Ident::new(&start_ident, proc_macro2::Span::call_site());

    quote! {
        #[doc(hidden)]
        #[#crate_::wasm_bindgen]
        pub fn #start_ident() {
            #crate_::WASM_INIT.with_borrow_mut(|v| {
                v.push(_on_process_start);
            })
        }
        fn _on_process_start(args: &#crate_::ProcessStartArgs) {
            fn on_process_start(args: &#crate_::ProcessStartArgs, handler: impl FnOnce(&#crate_::ProcessStartArgs)) {
                handler(args)
            }
            on_process_start(args, #closure)
        }
    }
    .into()
}
