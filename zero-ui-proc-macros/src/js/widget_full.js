// Script inserted in each widget module's full page after the user docs.
// It changes the page to highlight the widget property aspects.

document.addEventListener('DOMContentLoaded', function() {
    // patch title
    document.querySelector('h1 span').childNodes[0].nodeValue = 'Widget Module ';

    // remove property functions __pdoc_* and collect the summary of each.
    document.widget_property_summaries = {};
    let functions_h2 = document.querySelector('h2#functions.section-header');
    functions_h2.nextElementSibling.querySelectorAll('tr').forEach(function(tr) {
        let td = tr.querySelectorAll('td');
        if (td[0].innerText.includes('__pdoc_')) {
            document.widget_property_summaries[td[0].innerText] = td[1].innerHTML;
            tr.remove();
        } else if (td[0].innerText.includes('__p_')) {
            tr.remove();
        }
    });
    // remove functions section if it is empty.
    if (functions_h2.nextElementSibling.querySelector('tr') === null) {
        functions_h2.nextElementSibling.remove();
        functions_h2.remove();
    }

    // remove __inner_docs
    let modules_h2 = document.querySelector('h2#modules.section-header');
    modules_h2.nextElementSibling.querySelectorAll('tr').forEach(function(tr) {
        let td = tr.querySelectorAll('td');
        if (td[0].innerText.includes('__inner_docs')) {
            tr.remove();
        }
    });
    if (modules_h2.nextElementSibling.querySelector('tr') === null) {
        modules_h2.nextElementSibling.remove();
        modules_h2.remove();
    }

    // the header script ends up in the sidebar tooltip, remove it here.
    // note, the bad tooltips still show from an item page we don't control (like a struct in the same mod).
    document.querySelectorAll('div.block.fn li a, div.block.mod li a').forEach(function(a) {
        a.title = a.title.replace(/var local=doc.*/, '');
    });
});

window.addEventListener('message', function(a) {
    if (a.data.inner_docs !== undefined) {
        // insert the docs
        let inner_docs = document.createElement("div");
        inner_docs.innerHTML = a.data.inner_docs;
        let frame = document.getElementById('inner-docs-frame');
        frame.parentElement.insertAdjacentElement('afterend', inner_docs);
        frame.remove();

        // fix relative anchors
        inner_docs.querySelectorAll('a').forEach(function(a) {
            let href = a.getAttribute('href');
            if (href.startsWith('../')) {
                href = href.substr('../'.length);
                a.setAttribute('href', href);
            }
        });

        // fullfill summary requests for properties without help.
        inner_docs.querySelectorAll('.default-help').forEach(function(div) {
            let parse = document.createElement('div');
            parse.innerHTML = document.widget_property_summaries['__pdoc_' + div.getAttribute('data-ident')];
            div.replaceWith(parse.childNodes[0])
        });
    } else if (a.data.property_type !== undefined) {
        let inputs = a.data.property_type.fn.replace(/^\w+ = /, '');
        let target = this.document.getElementById(a.data.property_type.id).querySelector('span.ptype-request');
        target.removeAttribute('title');
        target.innerHTML = inputs;
    }
});