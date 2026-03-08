//! Generate language autonym code.

use icu::{
    casemap::{TitlecaseMapper, options::TitlecaseOptions},
    experimental::displaynames::{DisplayNamesOptions, LanguageDisplayNames, RegionDisplayNames},
    locale::LanguageIdentifier,
};

fn main() {
    let locales = [
        "af", "am", "ar", "as", "az", "be", "bg", "bn", "bs", "ca", "cs", "cy", "da", "de", "el", "en", "en-GB", "en-US", "es", "es-419",
        "es-ES", "et", "eu", "fa", "fi", "fil", "fr", "fr-CA", "ga", "gd", "gl", "gu", "he", "hi", "hr", "hu", "hy", "id", "is", "it",
        "ja", "ka", "kk", "km", "kn", "ko", "ky", "lo", "lt", "lv", "mk", "ml", "mn", "mr", "ms", "my", "nb", "ne", "nl", "nn", "or", "pa",
        "pl", "ps", "pt", "pt-BR", "pt-PT", "ro", "ru", "si", "sk", "sl", "sq", "sr", "sr-Latn", "sv", "sw", "ta", "te", "th", "tr", "uk",
        "ur", "uz", "vi", "zh", "zh-Hans", "zh-Hant", "zh-TW", "zu",
    ];

    if std::env::args().any(|a| a == "--locales") {
        for l in locales {
            println!("{l:?},");
        }
        return;
    }

    println!("// generated with cargo run --manifest-path tools/lang-autonym-gen/Cargo.toml");

    let options = DisplayNamesOptions::default();

    let tc_mapper = TitlecaseMapper::new();
    let tc_options = TitlecaseOptions::default();

    for l_str in locales {
        let l_id: LanguageIdentifier = l_str.parse().unwrap();

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
            _ => {
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
