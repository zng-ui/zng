// Hides tagged `macro_rules!` macros from crate root.

document.addEventListener('DOMContentLoaded', function () {
    let is_front_pg = false;
    document.querySelectorAll('h1').forEach(function (h) {
        if (h.innerText.trimStart().startsWith("Crate ")) {
            is_front_pg = true;
        }
    });

    if (is_front_pg) {
        // remove tagged macros.
        let removes = document.querySelectorAll('span[data-del-macro-root]');
        for (let remove of removes) {
            let div = remove.parentElement.parentElement;
            div.previousElementSibling.remove();
            div.remove();
        }
    }

    // remove empty macros section.
    let title = document.getElementById('macros');
    if (title != null && title.nextElementSibling.querySelector('a') == null) {
        title.nextElementSibling.remove();
        title.remove();

        let side_anchor = document.querySelector('a[href="#macros"]');
        if (side_anchor != null) {
            side_anchor.parentElement.remove();
        }
    }
})