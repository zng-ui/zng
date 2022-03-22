// Customizes widget module pages and widgets in mod lists.

document.addEventListener('DOMContentLoaded', function() {
    editWgtList();
    editWgtSideBar();
    editWgtPage();
});

window.addEventListener('message', function(a) {
    if (a.data.inner_docs !== undefined) {
        onDocsIframeLoaded(a.data.inner_docs);
    }
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
        document.querySelectorAll('details code').forEach(function(c) {
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

        let inner_docs_a = document.querySelector('a[href="constant.__DOCS.html"]');
        let consts_table = inner_docs_a.closest("div.item-table");
        inner_docs_a.remove();
        if (consts_table.querySelector('a') == null) {
            consts_table.previousElementSibling.remove();
            consts_table.remove();
            // sidepanel
            document.querySelector('a[href="#constants"]').remove();
        }
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

function onDocsIframeLoaded(docs) {
    let inner_docs = document.createElement("div");
    inner_docs.innerHTML = docs;
    inner_docs.getElementsByTagName('p')[0].remove();
    inner_docs.getElementsByTagName('p')[0].remove();
    let frame = document.getElementById('wgt-docs-iframe');
    frame.parentElement.insertAdjacentElement('afterend', inner_docs);
    frame.remove();

    editWgtPageSideBar();
}

// customize sidebar of widget module page.
function editWgtPageSideBar() {
    let sidebar = document.querySelector("div.sidebar-elems section");

    let mod_items = sidebar.querySelector('.block');
    let first_mod_item = mod_items.querySelector('a');
    if(first_mod_item != null) {
        let title = document.createElement('h3');
        title.classList.add("sidebar-title");
        let a = first_mod_item.cloneNode(true);
        a.innerText = "Module Items";
        title.appendChild(a);
        mod_items.insertBefore(title, mod_items.querySelector('ui'));
    } else {
        mod_items.remove();
        mod_items = null;
    }

    let widget_items_ul = document.createElement('ul');
    appendSidebarAnchor(widget_items_ul, "required-properties");
    appendSidebarAnchor(widget_items_ul, "normal-properties");
    appendSidebarAnchor(widget_items_ul, "state-properties");
    appendSidebarAnchor(widget_items_ul, "when-conditions");

    let first_widget_item = widget_items_ul.querySelector('a');
    if (first_widget_item != null) {
        let widget_items = document.createElement('div');
        widget_items.classList.add("block");

        let title = document.createElement('h3');
        title.classList.add("sidebar-title");

        let a = first_widget_item.cloneNode(true);
        a.innerText = "Widget Items";
        title.append(a);

        widget_items.appendChild(title);
        widget_items.appendChild(widget_items_ul);
        sidebar.insertBefore(widget_items, mod_items);
    }
}
function appendSidebarAnchor(ul, id) {
    let el = document.getElementById(id);
    if(el != null) {
        let li = document.createElement("li");
        let a = document.createElement("a");
        a.href = "#" + id;
        a.innerText = el.innerText;
        li.appendChild(a);
        ul.appendChild(li);
    }
}