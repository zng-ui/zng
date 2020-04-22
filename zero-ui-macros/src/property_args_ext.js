document.addEventListener("DOMContentLoaded", function() {
    var args = document.querySelector("#args_example pre");
    args.classList.add("rust");
    args.innerHTML = args.getElementsByTagName("code")[0].innerText;
});
window.addEventListener("message", function(e) {
    document.getElementById("args_example_load").remove();
    var full = document.createElement("div");
    full.innerHTML = e.data;
    var full = full.childNodes[0];
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
    full.remove();
});