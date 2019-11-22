#[macro_use]
extern crate derive_new;

#[macro_use]
pub mod core;
pub mod primitive;

pub mod app;

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
