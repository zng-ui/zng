fn main() {
    let hello = "Hello World!".to_owned();
    let other = "other".to_owned();

    // A (not possible currently: https://github.com/rust-lang/rust/issues/15701)
    foo(
        #[clone] move || {
            // `hello` is cloned before creating the closure, the clone is moved in,
            // `other` is moved in.
            println!("{} {}", hello.clone(), other);
        },
    );

    // B
    foo(
        clone_move! { |ctx, args| 
            let wns = ctx.services.req::<Windows>();
            wns.window(args.window_id);
            let hello = "";
            // can't known hello is not from the outside.
            println!("{} {}", hello.clone(), other);
        }
    );

    // C
    foo(
        #[clone_move(hello, |ctx, args|)] {
            // `hello` is cloned before creating the closure, the clone is moved in,
            // `other` is moved in.
            println!("{} {}", hello, other);

            // this works in stable, downside is that rust-analyzer does not autocomplete for `|ctx, args|`
            // and rustfmt moves the attribute above the block.
        }
    );

    // D
    foo(
        clone_move! { hello, .. |ctx, args|
            // closure block
        }
    );

    // manual
    foo(
        {
            let hello = hello.clone();
            move |ctx, args| {

            }
        }
    );

    // manual assisted
    foo(
        {
            clone!(hello, ..);// create another hello, etc.
            move |ctx, args| {
                
            }
        }
    );

    // current
    foo(
        enclose!((hello) move |ctx, args| {
            println!("{} {}", hello, other);
        })
    );

    // this code should compile.
    println!("len: {}", hello.len());
}

fn main_expanded() {
    foo({
        let __hello_clone = hello.clone();
        move || {
            println!("{} {}", __hello_clone, other)
        }
    });
}

fn foo(bar: impl Fn() + 'static) {
    bar()
}
