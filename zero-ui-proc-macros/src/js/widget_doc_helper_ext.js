document.addEventListener('DOMContentLoaded', function() {
    var table = document.querySelector('table');
    if(table !== null) {
        window.parent.postMessage(table.outerHTML, '*');
    }
});