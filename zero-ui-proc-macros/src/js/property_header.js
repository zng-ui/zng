// Script inserted at the start of each property function docs.
// In the functions list page it creates a new 'Property Functions' section.
var div = document.currentScript.closest('div');
if (document.property_fns === undefined) {
    document.property_fns = new Array(div);
    document.addEventListener('DOMContentLoaded', move_properties);
} else {
    document.property_fns.push(div);
}

function move_properties() {
    let functions = document.querySelector('h2#functions.section-header');
    if (functions !== null) {
        // we are in the functions list page:

        let property_fns = document.property_fns.filter(function(div) {
            return document.documentElement.contains(div);
        });

        if (property_fns.length != 0) {
            // create section header
            let pf = document.createElement('h2');
            pf.id = 'property-functions';
            pf.classList.add('section-header');
            let a = document.createElement('a');
            a.href = '#' + pf.id;
            a.innerHTML = 'Property Functions';
            pf.appendChild(a);
            functions.parentNode.insertBefore(pf, functions);

            // create section link in the sidebar
            let ul = document.querySelector('div.block.items ul');
            let side_functions = ul.querySelector('a[href=\u0022#functions\u0022]').parentNode;
            let li = document.createElement('li');
            li.appendChild(a.cloneNode(true));
            ul.insertBefore(li, side_functions);

            // create table
            let table = document.createElement('div');
            table.classList.add('item-table');
            property_fns.forEach(function(div) {
                table.appendChild(div.previousElementSibling);
                table.appendChild(div);
            });
            functions.parentNode.insertBefore(table, pf.nextSibling);

            // remove empty section
            if (functions.nextElementSibling.querySelector('tr') === null) {
                functions.nextElementSibling.remove();
                functions.remove();
                side_functions.remove();
            }
        }
    }

    // the header script ends up in the sidebar tooltip, remove it here.
    // note, the bad tooltips still show from an item page we don't control (like a struct in the same mod).
    document.querySelectorAll('div.block.fn li a, div.block.mod li a').forEach(function(a) {
        a.title = a.title.replace(/var div=doc.*/, '');
    });
}