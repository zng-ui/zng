// Script inserted at the start of each widget module docs.
// In the modules list page it creates a new "Widget Modules" section.

var local = document.currentScript.closest('tr');
if (local !== null) {
    if (document.widget_modules === undefined) {
        document.widget_modules = new Array(local);

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
                var table = document.createElement('table');
                table.style = 'display:block;';
                var tbody = document.createElement('tbody');
                document.widget_modules.forEach(function(tr) {
                    tbody.appendChild(tr);
                });
                document.widget_modules = null;
                table.appendChild(tbody);
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
                    a.title = a.title.replace(/var local=doc.*/, '');
                });
            }
        });
    } else {
        document.widget_modules.push(local);
    }
}