function on_example_load() {
    var load = document.getElementById("args_example_load");
    var full_doc = load.contentDocument || load.contentWindow.document;
    var full = full_doc.querySelector(".rust");
    var basic = document.getElementById("args_example");
    var first_ident = basic.querySelector(".ident").textContent + ':';
    var example_started = false;
    for (var fnode of full.childNodes) {
        if(!example_started) {
            example_started = fnode.textContent.includes(first_ident);
        }
        else if (fnode.nodeName === "A") {
            for (var target of basic.querySelectorAll("span.ident")) {
                if (target.textContent === fnode.textContent) {
                    target.replaceWith(fnode);
                    break;
                }
            }
        }
    }
    load.remove();
}