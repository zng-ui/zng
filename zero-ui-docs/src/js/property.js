// Customizes property function pages and properties in function lists.

document.addEventListener('DOMContentLoaded', function () {
    editPropPage();
    editPropList();
    editPropSideBar();
});

// edit the property function item page, changes title, declaration code and removes the property tag.
function editPropPage() {
    let is_fn_pg = false;
    let h1 = null;
    document.querySelectorAll('h1').forEach(function (h) {
        if (h.innerText.trimStart().startsWith("Function ")) {
            h1 = h;
            is_fn_pg = true;
        }
    });

    let is_prop_pg = false;
    let code = null;
    if (is_fn_pg) {
        document.querySelectorAll('code').forEach(function (c) {
            if (c.innerText == "property") {
                code = c;
                is_prop_pg = true;
            }
        })
    }

    if (is_prop_pg) {
        // edit page.

        code.remove();
        let txt = h1.childNodes[0].childNodes[0];
        h1.childNodes[0].replaceChild(document.createTextNode("Property "), txt);

        let decl_code = document.querySelector("pre.rust.fn");
        let fn_decl_code = decl_code.cloneNode(true);

        let as_fn_title = document.getElementById("as-function");
        let capture_only = as_fn_title == null;

        if (!capture_only) {
            as_fn_title.parentElement.insertBefore(fn_decl_code, as_fn_title.nextElementSibling);
        }

        editPropDecl(capture_only, decl_code, fn_decl_code);
    }
}
function editPropDecl(capture_only, fn, ffn) {
    // remove where section for editing the innerText
    let where = fn.querySelector('span.where');
    if (where !== null) {
        where.remove();
    }

    // edit the function source code to only show the property name and arguments.
    let m = fn.innerText.match(/(?<vis>pub.*)?fn (?<name>\w+)(?:<.+(?=>\()>)?\((?<inputs>.+)\).*/s).groups;
    let inputs = m.inputs;
    if (!capture_only) {
        inputs = inputs.replace(/\s*\w+: .+?(?=,\s*\w+: ),\n?/s, '');
    }

    if (inputs.match(/\w: /g).length > 1) {
        fn.innerText = `${m.vis}${m.name} = {${inputs}};`;
    } else {
        let input = inputs.trim().replace(/,$/, '').replace(/\w+: /, '');
        fn.innerText = `${m.vis}${m.name} = ${input};`;
    }

    // recreate the type anchors:
    let set = new Set();
    for (let a of ffn.getElementsByTagName('a')) {
        if (!set.has(a.innerText)) {
            fn.innerHTML = fn.innerHTML.replaceAll(a.innerText, a.outerHTML);
            set.add(a.innerText)
        }
    }

    // reapend where section
    if (where !== null) {
        fn.appendChild(where);
    }
}

// edit the Functions list of a module, creates a Properties section, removes property tag.
function editPropList() {
    let functions = document.getElementById("functions");
    if (functions == null) {
        return;
    }

    let tags = [];
    functions.nextElementSibling.querySelectorAll("code").forEach(function (c) {
        if (c.innerText == "property") {
            tags.push(c);
        }
    });

    if (tags.length == 0) {
        return;
    }

    let properties = document.getElementById("properties");
    if (properties == null) {
        properties = functions.cloneNode(true);
        properties.id = "properties";
        let pa = properties.querySelector("a");
        pa.href = "#properties";
        pa.innerText = "Properties";

        functions.parentElement.insertBefore(properties, functions);

        let properties_table = functions.nextElementSibling.cloneNode(false);
        functions.parentElement.insertBefore(properties_table, functions);

        // insert sidebar link
        let sidebarFns = document.querySelector("li a[href='#functions']").parentElement;
        let sidebarProps = sidebarFns.cloneNode(true);
        let sidebarPropsA = sidebarProps.querySelector("a");
        sidebarPropsA.innerText = "Properties";
        sidebarPropsA.href = "#properties";
        sidebarFns.parentElement.insertBefore(sidebarProps, sidebarFns);
    }

    let properties_table = properties.nextElementSibling;

    for (const tag of tags) {
        let row = tag.closest("div.item-row");
        tag.remove();
        properties_table.appendChild(row);
    }

    let functions_table = functions.nextElementSibling;
    if (functions_table.querySelector("a") == null) {
        functions_table.remove();
        functions.remove();

        let sidebarFns = document.querySelector("li a[href='#functions']").parentElement;
        sidebarFns.remove();
    }
}

