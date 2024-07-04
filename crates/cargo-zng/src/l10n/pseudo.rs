use std::borrow::Cow;

pub fn pseudo(s: &str) {
    fluent_pseudo_impl(s, false, false)
}

pub fn pseudo_mirr(s: &str) {
    fluent_pseudo_impl(s, true, false)
}

pub fn pseudo_wide(s: &str) {
    fluent_pseudo_impl(s, false, true)
}

fn fluent_pseudo_impl(s: &str, flipped: bool, elongate: bool) {
    pseudo_impl(s, |s| fluent_pseudo::transform(s, flipped, elongate))
}

fn pseudo_impl(s: &str, transform: impl Fn(&str) -> Cow<str>) {
    if s.contains('{') {
        let mut r = String::with_capacity(s.len());

        let mut start = 0;
        let mut depth = 0;
        for (i, c) in s.char_indices() {
            match c {
                '{' => {
                    if depth == 0 {
                        r.push_str(&transform(&s[start..i]));
                        start = i;
                    }
                    depth += 1;
                }
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        r.push_str(&s[start..=i]);
                        start = i + 1;
                    }
                }
                _ => {}
            }
        }
        if start < s.len() {
            if depth == 0 {
                r.push_str(&transform(&s[start..]));
            } else {
                r.push_str(&s[start..]);
            }
        }

        Cow::Owned(r)
    } else {
        transform(s)
    }
}
