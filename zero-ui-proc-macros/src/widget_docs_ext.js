document.addEventListener("DOMContentLoaded", function() {
    var ul = document.querySelector("div.block.items ul");
    if (ul === null) {
        var sidebar_elems = document.querySelector("div.sidebar-elems");
        var block_items = document.createElement("div");
        block_items.className = "block items";
        ul = document.createElement("ul");
        block_items.append(ul);
        sidebar_elems.prepend(block_items);
    }
    prepend_item("other-properties", "Other properties", ul);
    prepend_item("state-properties", "State properties", ul);
    prepend_item("provided-properties", "Provided properties", ul);
    prepend_item("required-properties", "Required properties", ul);
    var prop_help_requests = document.querySelectorAll("span.load-property-help");
    if (prop_help_requests.length > 0) {
        fetch("doc_helper/index.html").then(r => r.text()).then(t => {
            var help = document.createElement("document");
            help.outerHTML = t;
            fulfill_prop_help_requests(prop_help_requests, help.querySelector("table"));
        });
    }
});
function prepend_item(id, text, ul) {
    if (document.getElementById(id) !== null) {
        var li = document.createElement("li");
        li.innerHTML = `<a href="#${id}">${text}</a>`;
        ul.prepend(li);
    }
}
function fulfill_prop_help_requests(requests, help){
    var property_help = {};
    help.rows.forEach(row => {
        var property = row.querySelector("a.mod").innerHTML;
        var help  = row.querySelector("p");
        property_help[property] = help;
    });
    var requests = document.querySelectorAll("span.load-property-help");
    requests.forEach(req => {
        var property = req.getAttribute("data-property");
        req.innerHTML = property_help[property];
    });
}