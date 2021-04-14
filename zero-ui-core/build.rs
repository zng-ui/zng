use cfg_aliases::cfg_aliases;

fn main() {
    cfg_aliases! {
        pub_test: { any(test, doc, feature="pub_test") }
    }
}
