use std::borrow::Cow;

pub fn pseudo(dir: &str) {
    fluent_pseudo_impl(dir, false, false)
}

pub fn pseudo_mirr(dir: &str) {
    fluent_pseudo_impl(dir, true, false)
}

pub fn pseudo_wide(dir: &str) {
    fluent_pseudo_impl(dir, false, true)
}

fn fluent_pseudo_impl(dir: &str, flipped: bool, elongate: bool) {
    pseudo_impl(dir, |s| fluent_pseudo::transform(s, flipped, elongate))
}

fn pseudo_impl(dir: &str, transform: impl Fn(&str) -> Cow<str>) {
    todo!()
}
