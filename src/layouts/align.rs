use crate::core::Ui;
use crate::properties::{align, CENTER};

#[inline]
pub fn center(child: impl Ui) -> impl Ui {
    align::set(child, CENTER)
}
