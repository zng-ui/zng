// Customizes widget module pages and widgets in mod lists.

document.addEventListener('DOMContentLoaded', function () {
    editWgtList();
    editWgtSideBar();
    editWgtPage();
});

window.addEventListener('message', function (a) {
    if (a.data.inner_docs !== undefined) {
        onDocsIframeLoaded(a.data.inner_docs);
    }
});

// edit the widget mod item page, changes title and removes the widget tag.
function editWgtPage() {
    let is_mod_pg = false;
    let h1 = null;
    document.querySelectorAll('h1').forEach(function (h) {
        if (h.innerText.trimStart().startsWith("Module ")) {
            h1 = h;
            is_mod_pg = true;
        }
    });

    let is_wgt_pg = false;
    let code = null;
    if (is_mod_pg) {
        document.querySelectorAll('details code').forEach(function (c) {
            if (c.innerText == "widget") {
                code = c;
                is_wgt_pg = true;
            }
        })
    }

    if (is_wgt_pg) {
        // edit page.

        code.remove();
        let txt = h1.childNodes[0].childNodes[0];
        h1.childNodes[0].replaceChild(document.createTextNode("Widget "), txt);

        document.querySelectorAll('h2.location a').forEach(function (a) {
            a.innerText = a.innerText.replace("Module ", "Widget ");
        });

        let inner_docs_a = document.querySelector('a[href="__DOCS/index.html"]');
        let mods_table = inner_docs_a.closest("div.item-table");
        inner_docs_a.closest('div.item-row').remove();
        if (mods_table.querySelector('a') == null) {
            mods_table.previousElementSibling.remove();
            mods_table.remove();
            document.querySelector('a[href="#modules"]').remove();
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
    mods.nextElementSibling.querySelectorAll("code").forEach(function (c) {
        if (c.innerText == "widget") {
            tags.push(c);
        }
    });

    if (tags.length == 0) {
        return;
    }

    let widgets = document.getElementById("widgets");
    if (widgets == null) {
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
    mods.querySelector("ul").querySelectorAll("a").forEach(function (a) {
        if (a.title.startsWith("`widget` ")) {
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

    if (mods.querySelector("ul").querySelector("a") == null) {
        mods.remove();
    }
}

function onDocsIframeLoaded(docs) {
    let inner_docs = document.createElement("div");
    inner_docs.innerHTML = docs;
    inner_docs.getElementsByTagName('p')[0].remove();
    inner_docs.getElementsByTagName('p')[0].remove();

    inner_docs.querySelectorAll('a').forEach(function(a) {
        let href = a.getAttribute('href');
        if (href.startsWith('../')) {
            href = href.substring('../'.length);
            a.setAttribute('href', href);
        }
    });

    // convert headers to H5.
    let known_titles = [
        "required-properties",
        "normal-properties",
        "event-properties",
        "state-properties",
        "when-conditions",
    ];
    inner_docs.querySelectorAll('h2,h3,h4').forEach(function (ho) {
        if (!known_titles.includes(ho.id)) {
            let hn = document.createElement('h5');
            hn.innerHTML = ho.innerHTML;

            let id = ho.id;
            if (document.getElementById(id) != null) {
                id += "-inner-docs";
                hn.querySelector("a").href = "#" + id;
            }

            hn.id = id;
            ho.replaceWith(hn);
        }
    });

    // convert property title to H4
    inner_docs.querySelectorAll(".wp-title").forEach(function (s) {
        let place = s.closest('ul');

        s.classList.add("structfield");
        s.classList.add("small-section-header");

        let p_anchor = s.querySelector('a');
        if (p_anchor.href.includes("fn@")) {
            p_anchor.href = "#" + s.id;
        }
        let type_html = "";

        let code = p_anchor.querySelector('code');
        p_anchor.innerText = code.innerText;
        code.innerHTML = p_anchor.outerHTML + type_html;
        p_anchor.remove();
        s.querySelector('strong').appendChild(code);

        let a = document.createElement('a');
        a.classList.add("anchor");
        a.classList.add("field");
        a.href = "#" + s.id;

        s.prepend(a);

        let title = document.createElement("h4");
        while (s.firstChild) {
            title.appendChild(s.firstChild);
        }
        for (index = s.attributes.length - 1; index >= 0; --index) {
            title.attributes.setNamedItem(s.attributes[index].cloneNode());
        }
        title.style.overflowX = "visible";
        title.style.borderBottomWidth = "0";
        title.style.paddingBottom = "0";
        code.style.backgroundColor = "transparent";

        place.replaceWith(title);
    });

    // convert when conditions title to H4
    inner_docs.querySelectorAll("code").forEach(function (c) {
        if (!c.innerText.startsWith("when ")) {
            return;
        }
        let place = c.closest("ul");

        let title_id = c.innerText.replaceAll(/\s+/gm, '-').replaceAll(/[^a-zA-Z0-9_\-\.]/gm, '').toLowerCase();

        c.innerHTML = c.innerHTML.replaceAll(/self\.(\w+)/gm, 'self.<a href="#wp-$1">$1</a>');

        let title = document.createElement("h4");
        title.id = title_id;
        title.classList.add("ww-title");
        title.classList.add("structfield");
        title.classList.add("small-section-header");

        let title_anchor = document.createElement('a');
        title_anchor.classList.add("anchor");
        title_anchor.classList.add("field");
        title_anchor.href = "#" + title_id;

        title.appendChild(title_anchor);
        title.appendChild(c.parentElement);

        title.style.overflowX = "visible";
        title.style.borderBottomWidth = "0";
        title.style.paddingBottom = "0";
        c.style.backgroundColor = "transparent";

        place.replaceWith(title);
    });

    let frame = document.getElementById('wgt-docs-iframe');
    let doc_block = frame.parentElement;
    frame.replaceWith(inner_docs);
    frame.remove();
    while (inner_docs.childNodes.length > 0) {
        doc_block.appendChild(inner_docs.childNodes[0]);
    }

    editWgtPageSideBar();
    if (window.location.hash.length > 0) {
        // scroll to anchor
        window.location.href = window.location.href;
    }

    fetchPropTypes();
}

// customize sidebar of widget module page.
function editWgtPageSideBar() {
    let sidebar = document.querySelector("div.sidebar-elems section");

    let mod_items = sidebar.querySelector('.block');
    let first_mod_item = mod_items.querySelector('a');
    if (first_mod_item != null) {
        let title = document.createElement('h3');
        title.classList.add("sidebar-title");
        let a = first_mod_item.cloneNode(true);
        a.innerText = "Module Items";
        title.appendChild(a);
        mod_items.insertBefore(title, mod_items.querySelector('ul'));
    } else {
        mod_items.remove();
        mod_items = null;
    }

    let widget_items_ul = document.createElement('ul');
    appendSidebarAnchor(widget_items_ul, "required-properties");
    appendSidebarAnchor(widget_items_ul, "normal-properties");
    appendSidebarAnchor(widget_items_ul, "event-properties");
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
    if (el != null) {
        let li = document.createElement("li");
        let a = document.createElement("a");
        a.href = "#" + id;
        a.innerText = el.innerText;
        li.appendChild(a);
        ul.appendChild(li);
    }
}

// fetch HTML, parse and fix base URL.
function fetchHtml(url) {
    return fetch(url)
            .then(function(r) { return r.text(); })
            .then(function(html) {
                var parser = new DOMParser();
                var doc = parser.parseFromString(html, "text/html");
                let base = doc.createElement('base');
                base.setAttribute('href', url);
                doc.head.append(base);

                doc.querySelectorAll('a').forEach(function(a) {
                    a.setAttribute('href', a.href);
                });

                return doc;
            });
}

// fetch linked property pages and edit the types with the types.
function fetchPropTypes() {
    let current_page = window.location.href.split('#')[0];
    if (current_page.startsWith("file:///")) {
        return;
    }

    document.querySelectorAll('h4.wp-title').forEach(function (title) {
        let url = title.querySelector('a:not(.anchor)').href;

        if (url.startsWith(current_page)) {
            return;
        }

        url = url.replace('/index.html', '/__DOCS/index.html');
        fetchHtml(url).then(function(doc) {
            if (url.includes('#')) {
                resolvePropPage(title, url, doc);
            } else {
                copyPropType(title, doc);
            }
        });
    });
}
function resolvePropPage(title, url, doc) {
    let anchor = url.split('#')[1];

    let inner_title = doc.getElementById(anchor);
    if (inner_title == null) {
        return;
    }

    let inner_url = inner_title.querySelector('a:not(.anchor)').href.replace('/index.html', '/__DOCS/index.html');

    if (inner_url.includes("fn@")) {
        return;
    }

    fetchHtml(inner_url)
        .then(function(inner_doc) {
            if (inner_url.includes('#')) {
                resolvePropPage(title, inner_url, inner_doc);
            } else {
                copyPropType(title, inner_doc);
            }
        });
}
function copyPropType(title, doc) {
    let target = title.querySelector('code');
    let code = doc.querySelector('pre.rust.fn');
    target.appendChild(code);
    editPropDecl(false, code, code.cloneNode(true));

    let pre = document.createElement('pre');
    pre.classList.add('rust');
    pre.classList.add('fn');
    pre.appendChild(target.firstChild);
    pre.appendChild(document.createTextNode(' =' + code.childNodes[0].textContent.split('=', 2)[1]));
    pre.style.margin = "0";
    pre.style.padding = "0";
    pre.style.backgroundColor = "transparent";
    code.firstChild.remove();
    while (code.firstChild) {
        pre.appendChild(code.firstChild);
    }

    target.replaceWith(pre);

    code.remove();

    let docs_request = title.nextElementSibling.querySelector('span[data-fetch-docs]');
    if (docs_request != null) {
        let docs_target = docs_request.parentElement;
        docs_target.innerHTML = doc.querySelector('summary + .docblock p').innerHTML;
        docs_target.firstChild.remove(); // `property` tag
    }
}