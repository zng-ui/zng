document.querySelector('script[src$="/main.js"]').onload = function () {
    const defaultInitSidebarItems = window.initSidebarItems;
    window.initSidebarItems = function (items) {
        defaultInitSidebarItems(items);
        editPropSideBar();
    }
};

// edit the sidebar property function items, identified by their tooltip.
function editPropSideBar() {
    let functions = document.querySelector("div.sidebar-elems div.block.fn");
    if (functions == null) {
        return;
    }

    let prop_anchors = [];
    functions.querySelector("ul").querySelectorAll("a").forEach(function (a) {
        if (a.title.startsWith("`property` ")) {
            prop_anchors.push(a);
        }
    });

    if (prop_anchors.length == 0) {
        return;
    }

    let properties = document.createElement("div");
    properties.classList.add("block");
    properties.classList.add("fn");

    let properties_title = functions.querySelector("h3").cloneNode(true);
    let properties_title_a = properties_title.querySelector("a");
    properties_title_a.href = properties_title_a.href.replace("#functions", "#properties");
    properties_title_a.innerText = "Properties";
    properties.appendChild(properties_title);

    let properties_list = document.createElement("ul");
    for (const prop_anchor of prop_anchors) {
        prop_anchor.title = prop_anchor.title.replace("`property` ", "").trimStart();
        properties_list.appendChild(prop_anchor.parentElement);
    }

    properties.appendChild(properties_list);

    functions.parentElement.insertBefore(properties, functions);

    if (functions.querySelector("ul").querySelector("a") == null) {
        functions.remove();
    }
}
