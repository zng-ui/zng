document.addEventListener("DOMContentLoaded", function() {
    document.querySelectorAll("span[data-inherited]").forEach(span => {
        var anchor = "#wgproperty." + span.innerText;
        span.parentElement.href += anchor;
    });
});