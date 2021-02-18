// Script inserted in each widget module's full page after the user docs.
// It changes the page to highlight the widget property aspects.

document.addEventListener('DOMContentLoaded', function () {
    // TODO find all property functions TRs "__p_*"
    if (document.widget_property_fns !== undefined) {
        document.widget_property_fns.forEach(function (request) {
            // TODO move TR help for __p_{request.property} to request.target
        });
    }
});