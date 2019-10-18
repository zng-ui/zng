#[macro_use]
extern crate derive_new;

pub mod app;

#[macro_use]
pub mod core;
pub mod primitive;

///The enclose macro for easier cloning
#[macro_export]
macro_rules! enclose {
    ( ($( $x:ident ),*) $y:expr ) => {
        {
            $(let $x = $x.clone();)*
            $y
        }
    };
}
