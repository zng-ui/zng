(function () {
    addEventListener("DOMContentLoaded", function () {
        refactorProperties('implementations');
        this.document.querySelectorAll('h2').forEach(function (d) {
            if (d.id.startsWith('deref-methods')) {
                refactorProperties(d.id);
            }
        });
        refactorSidebar();
    });

    var PROPERTIES = new Set();

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
            let mixCut = derefIdPrefix.indexOf('%3C'); // <
            if (mixCut !== -1) {
                derefIdPrefix = derefIdPrefix.substring(0, mixCut);
            }
            derefFrom = ' from ' + implementations.querySelector('span a:nth-of-type(2)').outerHTML;
        }

        let properties = this.document.createElement('h2');
        properties.id = 'properties' + derefIdPrefix;
        properties.classList.add('small-section-header');
        properties.innerHTML = 'Properties' + derefFrom + '<a href="#' + properties.id + '" class="anchor">§</a>';

        let propertiesList = this.document.createElement('div');
        propertiesList.id = 'properties' + derefIdPrefix + '-list';

        // insert property sections before first impl sections.
        let insertPoint = this.document.querySelector('#properties-insert-pt');
        if (insertPoint == null) {
            insertPoint = this.document.createElement('div');
            insertPoint.id = 'properties-insert-pt';
            implementations.parentElement.insertBefore(insertPoint, implementations);
        }
        insertPoint.parentElement.insertBefore(propertiesList, insertPoint);
        insertPoint.parentElement.insertBefore(properties, propertiesList);

        propertiesList.innerHTML = "<div class='impl-items'></div>";
        propertiesList = propertiesList.firstChild;
        implementationsList.querySelectorAll("details.method-toggle").forEach(function (e) {
            let t = e.querySelector("strong[title='This method is a widget property']");
            if (t != null) {
                let mtdAnchor = e.querySelector('a.fn');
                let propertyName = mtdAnchor.innerText;
                if (PROPERTIES.has(propertyName)) {
                    // override
                    e.remove();
                } else {
                    PROPERTIES.add(propertyName);
                    t.remove();
                    let mtdSignature = mtdAnchor.parentElement;
                    // same sintax as `widget_impl!`
                    mtdSignature.innerHTML = mtdSignature.innerHTML.replace('<br>&nbsp;&nbsp;&nbsp;&nbsp;&amp;self,', '').replace('&amp;self, ', '') + ';';
                    propertiesList.appendChild(e);

                }

            }
        });

        // remove empty sections
        if (implementationsList.querySelector("section.method") == null) {
            implementationsList.remove();
            implementations.remove();
        } else {
            implementationsList.querySelectorAll("details.implementors-toggle").forEach(function (e) {
                if (e.querySelector("section.method") == null) {
                    e.remove();
                }
            });
        }
    }

    function refactorSidebar() {
        let sideBar = this.document.querySelector('div.sidebar-elems section');
        let repeats = new Set();

        sideBar.querySelectorAll('h3').forEach(function (e) {
            if (e.innerText == "Methods" || e.innerText.indexOf("Methods from") !== -1) {
                let mtdList = e.nextSibling;
                let propList = this.document.createElement('ul');
                mtdList.querySelectorAll('a').forEach(function (a) {
                    if (PROPERTIES.has(a.innerText)) {
                        if (repeats.has(a.innerText)) {
                            a.remove();
                        } else {
                            repeats.add(a.innerText);
                            propList.appendChild(a.parentElement);
                        }

                    }
                });
                if (propList.hasChildNodes()) {
                    propList.classList.add('block');

                    // insert property sections before first impl sections.
                    let insertPoint = this.document.querySelector('#properties-side-insert-pt');
                    if (insertPoint == null) {
                        insertPoint = this.document.createElement('div');
                        insertPoint.id = 'properties-side-insert-pt';
                        sideBar.insertBefore(insertPoint, e);
                    }

                    let title = this.document.createElement('h3');
                    let mtdsFrom = e.innerText.indexOf("Target=");
                    if (mtdsFrom !== -1) {
                        let cutStart = mtdsFrom + "Target=".length;
                        let cutEnd = e.innerText.indexOf("<", cutStart);
                        if (cutEnd === -1) {
                            cutEnd = e.innerText.length - 1;
                        }
                        let parentName = e.innerText.substring(cutStart, cutEnd);
                        title.innerHTML = '<a href="#properties-' + parentName + '">Properties from ' + parentName + '</a>';
                    } else {
                        title.innerHTML = '<a href="#properties">Properties</a>';
                    }
                    sideBar.insertBefore(title, insertPoint);
                    sideBar.insertBefore(propList, insertPoint);

                    if (!mtdList.hasChildNodes()) {
                        mtdList.remove();
                        e.remove();
                    }
                }
            }
        });
    }
})();