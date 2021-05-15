fn main() {
    let hello = "Hello World!".to_owned();
    let other = "other".to_owned();

    // A:
    foo(
        #[clone] || {
            // `hello` is cloned before creating the closure, the clone is moved in,
            // `other` is not moved, just referenced.
            println!("{} {}", hello.clone(), other);
        },
    );
    foo(
        #[clone] move || {
            // `hello` behaves the same here, but..
            // `other` is moved in.
            println!("{} {}", hello.clone(), other);
        },
    );

    // this code should compile.
    println!("len: {}", hello.len());
}

fn foo(bar: impl Fn() + 'static) {
    bar()
}
