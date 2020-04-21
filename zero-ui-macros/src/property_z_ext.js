document.addEventListener("DOMContentLoaded", function() {
    var full = document.querySelector(".rust");
    if(full !== undefined) {
        window.parent.postMessage(full.outerHTML, "*");
    }
});