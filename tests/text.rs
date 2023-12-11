use std::fmt::Write as _;

use zero_ui::{
    font::SegmentedText,
    layout::TextSegmentKind,
    wgt_prelude::{LayoutDirection, Txt},
};

#[test]
fn emoji_segs() {
    let tests = std::fs::read_to_string("../examples/res/text/unicode-emoji-15.0/emoji-test.txt").unwrap();

    let mut errors = String::new();
    let mut error_count = 0;

    for line in tests.lines() {
        if line.starts_with('#') || line.is_empty() {
            continue;
        }

        let line = if let Some((_, test)) = line.split_once(';') {
            if !test.starts_with(" fully-qualified") && !test.starts_with(" component") {
                continue;
            }
            test
        } else {
            continue;
        };

        if let Some((_, test)) = line.split_once('#') {
            let txt = SegmentedText::new(Txt::from_str(test), LayoutDirection::LTR);
            let k: Vec<_> = txt.segs().iter().map(|s| s.kind).take(3).collect();

            if k != vec![TextSegmentKind::Space, TextSegmentKind::Emoji, TextSegmentKind::Space] {
                error_count += 1;
                if error_count <= 20 {
                    let _ = writeln!(&mut errors, "{test}");
                }
            }
        }
    }

    if !errors.is_empty() {
        if error_count > 20 {
            let _ = writeln!(&mut errors, "\n..and {} more errors", error_count - 20);
        }
        panic!("\n\n{errors}");
    }
}
