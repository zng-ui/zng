document.addEventListener('DOMContentLoaded', function() {
    var full = document.querySelector('.rust');
    if(full !== null) {
        window.parent.postMessage(full.outerHTML, '*');
    }
});