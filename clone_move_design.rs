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
        }
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
