// Script inserted in each widget module's __inner_docs module to send the
// inner docs back yo the main page.

function default_help(property) {
    // placeholder
}

function load_default_help() {
    // placeholder
}

document.addEventListener('DOMContentLoaded', function() {
    let message = {
        inner_docs: document.getElementById("inner-docs").innerHTML,
    };
    window.parent.postMessage(message, "*")
})