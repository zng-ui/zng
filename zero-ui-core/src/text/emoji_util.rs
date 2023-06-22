use icu_properties::sets;

pub(super) fn maybe_emoji(c: char) -> bool {
    sets::load_emoji(&icu_testdata::unstable()).unwrap().as_borrowed().contains(c)
}

pub(super) fn definitely_emoji(c: char) -> bool {
    sets::load_emoji_presentation(&icu_testdata::unstable())
        .unwrap()
        .as_borrowed()
        .contains(c)
        || is_modifier(c)
}

pub(super) fn is_modifier(c: char) -> bool {
    sets::load_emoji_modifier(&icu_testdata::unstable())
        .unwrap()
        .as_borrowed()
        .contains(c)
}

pub(super) fn is_component(c: char) -> bool {
    sets::load_emoji_component(&icu_testdata::unstable())
        .unwrap()
        .as_borrowed()
        .contains(c)
}

/*
Loaded data is !Send+!Sync so we probably don't need to cache it.

The "icu_testdata" includes the stuff we need, plus a lot of useless data, there is a complicated way to
optmize this, but they are about to release embedded data, so we wait.

see: see https://github.com/unicode-org/icu4x/issues/3529

 */
