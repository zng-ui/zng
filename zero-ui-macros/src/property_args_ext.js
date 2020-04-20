function on_example_load_old() {
    var load = document.getElementById("args_example_load");
    var full = load.contentDocument || load.contentWindow.document;
    var full = full.querySelector(".rust");
    var full = Array.from(full.childNodes);
    var basic = document.getElementById("args_example");
    var first_ident = basic.querySelector(".ident").textContent + ':';
    var example_start = full.find(n => n.textContent.includes(first_ident));
    var i = full.indexOf(example_start);
    for (i; i < full.length; i++) {
        if (full[i].nodeName === "A") {
            for (var target of basic.querySelectorAll("span.ident")) {
                if (target.textContent === full[i].textContent) {
                    var node = full[i];
                    node.style.cssText = document.defaultView.getComputedStyle(node, "").cssText;
                    target.replaceWith(node);
                    break;
                }
            }
        }
    }
    load.remove();
}
function on_example_load() {
    var load = document.getElementById("args_example_load");
    var full_doc = load.contentDocument || load.contentWindow.document;
    var full = full_doc.querySelector(".rust");
    var basic = document.getElementById("args_example");
    var first_ident = basic.querySelector(".ident").textContent + ':';
    var example_started = false;
    load.style.display = 'block';
    for (var fnode of full.childNodes) {
        if(!example_started) {
            example_started = fnode.textContent.includes(first_ident);
        }
        else if (fnode.nodeName === "A") {
            for (var target of basic.querySelectorAll("span.ident")) {
                if (target.textContent === fnode.textContent) {
                    var a = fnode;
                    console.log(full_doc.defaultView.getComputedStyle(a, "").color);
                    a.style.color = full_doc.defaultView.getComputedStyle(a, "").color;
                    target.replaceWith(a);
                    break;
                }
            }
        }
    }
    load.remove();
}