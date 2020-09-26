if (document.property_modules === undefined) {
    document.property_modules = true;
    var local = document.currentScript.parentElement;
    document.addEventListener('DOMContentLoaded', function () {
        var modules = document.querySelector('h2#modules.section-header');
        if (modules !== null) {
            var pm = document.querySelector('h2#property-modules.section-header');
            pm = document.createElement('h2');
            pm.id = 'property-modules';
            pm.classList.add('section-header');
            pm.innerHTML = 'Property Modules';
            modules.parentNode.insertBefore(pm, modules.nextElementSibling.nextSibling);
        }
    });
}