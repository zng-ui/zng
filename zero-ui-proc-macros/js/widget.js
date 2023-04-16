addEventListener("DOMContentLoaded", function () {
    refactorProperties('implementations');
    this.document.querySelectorAll('h2').forEach(function(d) {
        if (d.id.startsWith('deref-methods')) {
            refactorProperties(d.id);
        }
    });
});

function refactorProperties(sectionId) {
    let implementations = this.document.getElementById(sectionId);
    if (implementations == null) {
        return;
    }

    let implementationsList = implementations.nextSibling;
    
    let isDeref = implementations.innerHTML.indexOf("Methods from") !== -1;
    let derefFrom = '';
    let derefIdPrefix = '';
    if (isDeref) {
        derefIdPrefix = sectionId.replace('deref-methods', '');
        derefFrom = ' from ' + implementations.querySelector('span a:nth-of-type(2)').outerHTML;
    }

    let properties = this.document.createElement('h2');
    properties.id = 'properties' + derefIdPrefix;
    properties.classList.add('small-section-header');
    properties.innerHTML = 'Properties' + derefFrom + '<a href="#' + properties.id + '" class="anchor">§</a>';

    let propertiesList = this.document.createElement('div');
    propertiesList.id = 'properties' + derefIdPrefix + '-list';

    // insert all property sections before first impl section.
    let insertPoint = this.document.querySelector('#properties-insert-pt');
    if (insertPoint == null) {
        insertPoint = this.document.createElement('div');
        insertPoint.id = 'properties-insert-pt';
        implementations.parentElement.insertBefore(insertPoint, implementations);
    }
    insertPoint.parentElement.insertBefore(propertiesList, insertPoint);
    insertPoint.parentElement.insertBefore(properties, propertiesList);

    implementationsList.querySelectorAll("div.impl-items").forEach(function (e) {
        let t = e.querySelector("strong[title='This method is a widget property']");
        if (t != null) {
            t.remove();
            e.style.marginBottom = "0.75em";
            propertiesList.appendChild(e);
        }
    });

    // remove empty sections
    implementationsList.querySelectorAll("section.impl").forEach(function (e) {
        let details = e.parentElement.parentElement;
        if (details.querySelector("section.method") == null) {
            details.remove();
        }
    });
    if (!implementationsList.hasChildNodes()) {
        implementationsList.remove();
        implementations.remove();
    }
}