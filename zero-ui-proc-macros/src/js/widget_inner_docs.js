// Script inserted in each widget module's __inner_docs module to send the
// inner docs back yo the main page.

document.addEventListener('DOMContentLoaded', function() {
    let message = {
        inner_docs: document.getElementById("inner-docs").innerHTML,
    };
    window.parent.postMessage(message, "*")
})