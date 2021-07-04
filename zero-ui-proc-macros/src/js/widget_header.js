// Script inserted at the start of each widget module docs.
// In the modules list page it creates a new "Widget Modules" section.

var div = document.currentScript.closest('div');
if (div !== null) { // avoid using && here because rustdoc breaks it.
    if (div.classList.contains('item-right')) {
        if (document.widget_modules === undefined) {
            document.widget_modules = new Array(div);

            document.addEventListener('DOMContentLoaded', function() {
                var modules = document.querySelector('h2#modules.section-header');
                if (modules !== null) {
                    // create section header
                    var pm = document.createElement('h2');
                    pm.id = 'widget-modules';
                    pm.classList.add('section-header');
                    var a = document.createElement('a');
                    a.href = '#' + pm.id;
                    a.innerHTML = 'Widget Modules';
                    pm.appendChild(a);
                    modules.parentNode.insertBefore(pm, modules.nextElementSibling.nextSibling);

                    // create section link in the sidebar
                    var ul = document.querySelector('div.block.items ul');
                    var side_modules = ul.querySelector('a').parentNode;
                    var li = document.createElement('li');
                    li.appendChild(a.cloneNode(true));
                    ul.insertBefore(li, side_modules.nextSibling);

                    // create table
                    var table = document.createElement('div');
                    table.classList.add('item-table');
                    document.widget_modules.forEach(function(div) {
                        table.appendChild(div.previousElementSibling);
                        table.appendChild(div);
                    });
                    document.widget_modules = null;
                    modules.parentNode.insertBefore(table, pm.nextSibling);

                    // remove empty modules
                    if (modules.nextElementSibling.querySelector('tr') === null) {
                        modules.nextElementSibling.remove();
                        modules.remove();
                        side_modules.remove();
                    }

                    // the header script ends up in the sidebar tooltip, remove it here.
                    // note, the bad tooltips still show from an item page we don't control (like a struct in the same mod).
                    document.querySelectorAll('div.block.fn li a, div.block.mod li a').forEach(function(a) {
                        a.title = a.title.replace(/var div=doc.*/, '');
                    });
                }
            });
        } else {
            document.widget_modules.push(div);
        }
    }
}