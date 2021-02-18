// Script inserted at the start of each property function docs.
// In the functions list page it creates a new "Property Functions" section.
var local = document.currentScript.closest('tr');
if (document.property_fns === undefined) {
    document.property_fns = new Array(local);
    document.addEventListener('DOMContentLoaded', move_properties);
} else {
    document.property_fns.push(local);
}
function move_properties() {
    let functions = document.querySelector('h2#functions.section-header');
    if (functions !== null) {
        // we are in the functions list page:

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
        let table = document.createElement('table');
        table.style = 'display:block;';
        let tbody = document.createElement('tbody');
        document.property_fns.forEach(function (tr) {
            tbody.appendChild(tr);
        });
        document.property_fns = null;
        table.appendChild(tbody);
        functions.parentNode.insertBefore(table, pf.nextSibling);

        // remove empty section
        if (functions.nextElementSibling.querySelector('tr') === null) {
            functions.nextElementSibling.remove();
            functions.remove();
            side_functions.remove();
        }
    }
}