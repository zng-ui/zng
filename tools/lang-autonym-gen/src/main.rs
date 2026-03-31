//! Generate language autonym code.

use icu::{
    casemap::{TitlecaseMapper, options::TitlecaseOptions},
    experimental::displaynames::{DisplayNamesOptions, LanguageDisplayNames, RegionDisplayNames},
};
use unic_langid::LanguageIdentifier;
// this does not support "pseudo" language
use icu::locale::LanguageIdentifier as StrictLanguageIdentifier;

fn main() {
    let locales = include_str!("locales.txt");

    if std::env::args().any(|a| a == "--locales") {
        for l in locales.lines() {
            print!("{l:?},");
        }
        println!();
        return;
    }

    println!("// generated with cargo run --manifest-path tools/lang-autonym-gen/Cargo.toml");

    let options = DisplayNamesOptions::default();

    let tc_mapper = TitlecaseMapper::new();
    let tc_options = TitlecaseOptions::default();

    for l_str in locales.lines() {
        let l_id: LanguageIdentifier = l_str.parse().unwrap_or_else(|e| panic!("{e}, {l_str}"));
        print!(
            r#"("{}", "{}", "{}") => "#,
            l_id.language.as_str(),
            l_id.script.as_ref().map(|s| s.as_str()).unwrap_or(""),
            l_id.region.as_ref().map(|r| r.as_str()).unwrap_or("")
        );

        match l_str {
            // name+script to disambiguate as the language name is 中文 in both scripts
            "zh-Hans" => print!(r#"("简体中文", "")"#),
            "zh-Hant" => print!(r#"("繁體中文", "")"#),
            // Hant implied for Taiwan
            "zh-TW" => print!(r#"("繁體中文", "台灣")"#),
            // pseudo
            "pseudo" => print!(r#"("Ƥşeuḓo", "")"#),
            "pseudo-Mirr" => print!(r#"("Ԁsǝnpo-Wıɹɹoɹǝp", "")"#),
            "pseudo-Wide" => print!(r#"("Ƥşeeuuḓoo-Ẇiḓee", "")"#),
            _ => {
                let l_id: StrictLanguageIdentifier = l_str.parse().unwrap_or_else(|e| panic!("{e}, {l_str}"));
                let lang_dn = LanguageDisplayNames::try_new(l_id.clone().into(), options).unwrap();
                let name = lang_dn.of(l_id.language).unwrap_or(l_id.language.as_str());

                let name = tc_mapper.titlecase_segment_to_string(name, &l_id, tc_options);
                print!(r#"("{name}", "#);

                if let Some(r) = l_id.region {
                    let reg_dn = RegionDisplayNames::try_new(l_id.clone().into(), options).unwrap();
                    let region = reg_dn.of(r).unwrap_or(r.as_str());
                    print!(r#""{region}")"#);
                } else {
                    print!("\"\")");
                }
            }
        }

        println!(",");
    }
}
