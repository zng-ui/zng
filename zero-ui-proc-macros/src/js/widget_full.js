// Script inserted in each widget module's full page after the user docs.
// It changes the page to highlight the widget property aspects.

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
    // patch title
    document.querySelector("h1 span").childNodes[0].nodeValue = "Widget Module ";

    // remove property functions __p_* and collect the summary of each.
    let summaries = {};
    let functions_h2 = document.querySelector('h2#functions.section-header');
    functions_h2.nextElementSibling.querySelectorAll("tr").forEach(function(tr) {
        let td = tr.querySelectorAll("td");
        if (td[0].innerText.includes("__p_")) {
            td[1].querySelector("script").remove();
            summaries[td[0].innerText] = td[1].innerHTML;
            tr.remove();
        }
    });
    // remove functions section if it is empty.
    if (functions_h2.nextElementSibling.querySelector("tr") === null) {
        functions_h2.nextElementSibling.remove();
        functions_h2.remove();
    }
    // fullfill summary requests for properties without help.
    if (document.widget_property_fns !== undefined) {
        let parse = document.createElement("div");
        document.widget_property_fns.forEach(function(request) {
            parse.innerHTML = summaries['__p_' + request.property];
            request.target.prepend(parse.firstChild);
        });
    }

    // the header script ends up in the sidebar tooltip, remove it here.
    // note, the bad tooltips still show from an item page we don't control (like a struct in the same mod).
    document.querySelectorAll('div.block.fn li a, div.block.mod li a').forEach(function(a) {
        a.title = a.title.replace(/var local=doc.*/, '');
    });
});