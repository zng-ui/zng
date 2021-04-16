fn main() {
    test::worked();
}

mod test {
    //! Hello

    lol!();

    use super::other::lol;
}

mod other {
    #[macro_export]
    macro_rules! fucks_sake {
        () => {
            pub fn worked() {
                println!("Worked!")
            }
        };
    }

    pub use crate::fucks_sake as lol;
}
