// Script inserted in each widget module's __inner_docs module to send the
// inner docs back yo the main page.

function default_help(property) {
    if (document.widget_property_fns === undefined) {
        document.widget_property_fns = [];
    }
    document.widget_property_fns.push({
        property: property,
        target: document.currentScript.parentNode
    });
}

document.addEventListener('DOMContentLoaded', function() {
    let message = {
        docs: document.getElementById("inner-docs"),
        default_requests: document.widget_property_fns
    };
    window.parent.postMessage(message, "*")
})