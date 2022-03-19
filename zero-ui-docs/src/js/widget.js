// Customizes widget module pages and widgets in mod lists.

document.addEventListener('DOMContentLoaded', function() {
    editWgtPage();
    editWgtList();
    editWgtSideBar();
});

// edit the widget mod item page, changes title and removes the widget tag.
function editWgtPage() {
    let is_mod_pg = false;
    let h1 = null;
    document.querySelectorAll('h1').forEach(function(h) {
        if (h.innerText.trimStart().startsWith("Module ")) {
            h1 = h;
            is_mod_pg = true;
        }
    });

    let is_wgt_pg = false;
    let code = null;
    if (is_mod_pg) {
        document.querySelectorAll('code').forEach(function(c) {
            if (c.innerText == "widget") {
                code = c;
                is_wgt_pg = true;
            }
        })
    }

    if(is_wgt_pg) {
        // edit page.

        code.remove();
        let txt = h1.childNodes[0].childNodes[0];
        h1.childNodes[0].replaceChild(document.createTextNode("Widget "), txt);

        document.querySelectorAll('h2.location a').forEach(function(a) {
            a.innerText = a.innerText.replace("Module ", "Widget ");
        });
    }
}

// edit the Modules list of a module, creates a Widgets section, removes widget tag.
function editWgtList() {
    let mods = document.getElementById("modules");
    if (mods == null) {
        return;
    }

    let tags = [];
    mods.nextElementSibling.querySelectorAll("code").forEach(function(c) {
        if (c.innerText == "widget") {
            tags.push(c);
        }
    });

    if(tags.length == 0) {
        return;
    }

    let widgets = document.getElementById("widgets");
    if(widgets == null) {
        widgets = mods.cloneNode(true);
        widgets.id = "widgets";
        let pa = widgets.querySelector("a");
        pa.href = "#widgets";
        pa.innerText = "Widgets";
        let mods_table = mods.nextElementSibling;

        mods.parentElement.insertBefore(widgets, mods_table.nextElementSibling);

        let widgets_table = mods_table.cloneNode(false);
        mods.parentElement.insertBefore(widgets_table, widgets.nextElementSibling);
    
        // insert sidebar link
        let sidebarMods = document.querySelector("li a[href='#modules']").parentElement;
        let sidebarWgts = sidebarMods.cloneNode(true);
        let sidebarWgtsA = sidebarWgts.querySelector("a");
        sidebarWgtsA.innerText = "Widgets";
        sidebarWgtsA.href = "#widgets";
        sidebarMods.parentElement.insertBefore(sidebarWgts, sidebarMods.nextElementSibling);
    }

    let widgets_table = widgets.nextElementSibling;

    for (const tag of tags) {
        let row = tag.closest("div.item-row");
        tag.remove();
        widgets_table.appendChild(row);
    }

    let mods_table = mods.nextElementSibling;
    if (mods_table.querySelector("a") == null) {
        mods_table.remove();
        mods.remove();
    }
}

// edit the sidebar widget mod items, identified by their tooltip.
function editWgtSideBar() {
    let mods = document.querySelector("div.sidebar-elems div.block.mod");
    if (mods == null) {
        return;
    }

    let wgt_anchors = [];
    mods.querySelector("ul").querySelectorAll("a").forEach(function(a) {
        if(a.title.startsWith("`widget` ")) {
            wgt_anchors.push(a);
        }
    });

    if (wgt_anchors.length == 0) {
        return;
    }

    let widgets = document.createElement("div");
    widgets.classList.add("block");
    widgets.classList.add("mod");

    let widgets_title = mods.querySelector("h3").cloneNode(true);
    let widgets_title_a = widgets_title.querySelector("a");
    widgets_title_a.href = widgets_title_a.href.replace("#modules", "#widgets");
    widgets_title_a.innerText = "Widgets";
    widgets.appendChild(widgets_title);

    let widgets_list = document.createElement("ul");
    for (const prop_anchor of wgt_anchors) {
        prop_anchor.title = prop_anchor.title.replace("`widget` ", "").trimStart();
        widgets_list.appendChild(prop_anchor.parentElement);
    }

    widgets.appendChild(widgets_list);

    mods.parentElement.insertBefore(widgets, mods.nextElementSibling);

    if(mods.querySelector("ul").querySelector("a") == null) {
        mods.remove();
    }
}