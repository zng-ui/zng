#[widget($crate::foo)]
pub mod foo {
    properties! {
        a_property;

        /// This property is set only when `a_property` is and it is a mapping of the a_property.
        b_property = 1 + self.a_property + 3;
    }
}

// # Can we allow handler capture too?

#[widget($crate::foo)]
pub mod foo {
    properties! {
        a_property;

        /// This property is set only when `a_property` is and it is a mapping of the a_property.
        b_property = hn!(self.a_property, |ctx, _| {
            println!(a_property.get(ctx));
        });
    }
}

// We can't reuse the the `when` code for handlers because they are not allowed in `when`.