# About

This directory contains JavaScript files are used to extend the `rustdoc` generated documentation.

The files are inserted by the proc-macros using `format!("<script>{}</script>", include_str!("js/*.js")`.

# Limitations

## No Empty Lines

The files cannot have empty lines, `rustdoc/markdown` HTML detection does not work over empty lines.

## No Fetching

The files cannot use `fetch` because its common to open documentation from local files and CORS blocks it (origin null).

To circumvent this issue load the extra content using `iframe` elements and use the `window.message` event to send data from inside the frame to the outside.