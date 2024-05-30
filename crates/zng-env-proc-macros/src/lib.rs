#![doc(html_favicon_url = "https://raw.githubusercontent.com/zng-ui/zng/master/examples/res/image/zng-logo-icon.png")]
#![doc(html_logo_url = "https://raw.githubusercontent.com/zng-ui/zng/master/examples/res/image/zng-logo.png")]
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
            return quote! { compile_error!(#msg) }.into();
        }
    };
    let manifest_str = match std::fs::read_to_string(&manifest) {
        Ok(s) => s,
        Err(e) => {
            let msg = format!("cannot read `{}`, {e}", manifest.display());
            return quote! { compile_error!(#msg) }.into();
        }
    };

    let m: Manifest = match toml::from_str(&manifest_str) {
        Ok(m) => m,
        Err(e) => {
            let msg = format!("cannot parse Cargo.toml manifest, {e}");
            return quote! { compile_error!(#msg) }.into();
        }
    };
    let p_name = m.package.name;
    let c_name = p_name.replace('-', "_");
    let p_authors = m.package.authors;
    let major = m.package.version.major;
    let minor = m.package.version.minor;
    let patch = m.package.version.patch;
    let pre = m.package.version.pre.to_string();
    let build = m.package.version.build.to_string();
    let desc = m.package.description.unwrap_or_default();
    let home = m.package.homepage.unwrap_or_default();
    let mut app = "";
    let mut org = "";
    let mut qualifier = "";
    let mut has_about = false;

    if let Some(m) = m
        .package
        .metadata
        .as_ref()
        .and_then(|m| m.zng.as_ref())
        .and_then(|z| z.about.as_ref())
    {
        has_about = true;
        app = m.app.as_deref().unwrap_or_default();
        org = m.org.as_deref().unwrap_or_default();
        qualifier = m.qualifier.as_deref().unwrap_or_default();
    }
    if app.is_empty() {
        app = &p_name;
    }
    if org.is_empty() {
        org = p_authors.first().map(|s| s.as_str()).unwrap_or_default();
    }

    /*
    pub fn macro_new(
        pkg_name: &'static str,
        pkg_authors: &[&'static str],
        crate_name: &'static str,
        (major, minor, patch, pre, build): (u64, u64, u64, &'static str, &'static str),
        app: &'static str,
        org: &'static str,
        qualifier: &'static str,
        description: &'static str,
        homepage: &'static str,
    )
     */
    quote! {
        #crate_::init(#crate_::About::macro_new(
            #p_name,
            &[#(#p_authors),*],
            #c_name,
            (#major, #minor, #patch, #pre, #build),
            #app,
            #org,
            #qualifier,
            #desc,
            #home,
            #has_about,
        ))
    }
    .into()
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
    authors: Box<[String]>,
    metadata: Option<Metadata>,
}
#[derive(serde::Deserialize)]
struct Metadata {
    zng: Option<Zng>,
}
#[derive(serde::Deserialize)]
struct Zng {
    about: Option<MetadataAbout>,
}
#[derive(serde::Deserialize)]
struct MetadataAbout {
    app: Option<String>,
    org: Option<String>,
    qualifier: Option<String>,
}
