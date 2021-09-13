# About

This test demonstrates splitting the "View-Process" from the "App-Process" so that the `split-app`
crate does not need to build the windowing and rendering crates. The test is successful if both crates build.

# Run

Steps to run:

1 - Change directory to `test-crates/split-app`.
2 - Run the app using `cargo run`.

# Notes

This test does not use a workspace deliberately, features are unified for crates in a workspace so the `"full"`
feature ends-up enabled in the `split-app` anyway. You should use a workspace in a real project if the view and app
crates are in the same repository.

You can have more then one app crate use the same View-Process executable, so you can have a suite of apps
that share the same view-process.

Note that in Windows the "Task Manager" highlights the process that creates windows, so the branding metadata
of the `split-view.exe` file is something the user may see.