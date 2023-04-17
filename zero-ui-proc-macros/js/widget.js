(function () {
    addEventListener("DOMContentLoaded", async function () {
        let inherits = [];
        await refactorDocument(this.document, inherits, new Set());
        mergeInherits(inherits);
    });

    var PROPERTIES = new Set();

    // * `doc` - The document, can be this.document or a fetched doc.
    // * `inherits` - Array of `{ link, page }`, link is HTML str of the parent link, .
    // * `fetchUrls` - Set of fetch URLs, used to avoid infinite recursion.
    async function refactorDocument(doc, inherits, fetchUrls) {
        refactorSections(doc, fetchUrls);
        refactorSidebar(doc);
        await fetchInherits(doc, inherits, fetchUrls);
    }

    function refactorSections(doc, fetchUrls) {
        refactorProperties(doc, 'implementations');
        doc.querySelectorAll('h2').forEach(function (d) {
            if (d.id.startsWith('deref-methods')) {
                let skipFirst = true;
                let insertPoint = d.nextElementSibling.nextElementSibling;
                d.querySelectorAll('a.struct').forEach(function (a) {
                    if (skipFirst) {
                        skipFirst = false;
                        return;
                    }

                    if (fetchUrls.has(a.href)) {
                        return;
                    }
                    fetchUrls.add(a.href);

                    // unloaded mix-in parent

                    let title = doc.createElement('h2');
                    title.innerHTML = 'Inherits from ' + a.outerHTML;
                    title.classList.add('inherit-fetch');
                    insertPoint.parentElement.insertBefore(title, insertPoint);

                    let section = doc.createElement('div');
                    section.innerText = 'Loading...';
                    insertPoint.parentElement.insertBefore(section, insertPoint);

                    insertPoint = section.nextElementSibling;
                    a.remove();
                });
                refactorProperties(doc, d.id);
            }
        });
    }

    function refactorProperties(doc, sectionId) {
        let implementations = doc.getElementById(sectionId);
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

        let properties = doc.createElement('h2');
        properties.id = 'properties' + derefIdPrefix;
        properties.classList.add('small-section-header');
        properties.innerHTML = 'Properties' + derefFrom + '<a href="#' + properties.id + '" class="anchor">§</a>';

        let propertiesList = doc.createElement('div');
        propertiesList.id = 'properties' + derefIdPrefix + '-list';

        // insert property sections before first impl sections.
        let insertPoint = doc.querySelector('#properties-insert-pt');
        if (insertPoint == null) {
            insertPoint = doc.createElement('div');
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
                    mtdSignature.innerHTML = mtdSignature.innerHTML
                        .replace('fn ', '')
                        .replace('<br>&nbsp;&nbsp;&nbsp;&nbsp;&amp;self,', '')
                        .replace('&amp;self, ', '') + ';';
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

    function refactorSidebar(doc) {
        let sideBar = doc.querySelector('div.sidebar-elems section');
        let repeats = new Set();

        sideBar.querySelectorAll('h3').forEach(function (e) {
            if (e.innerText == "Methods" || e.innerText.indexOf("Methods from") !== -1) {
                let mtdList = e.nextSibling;
                let propList = doc.createElement('ul');
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
                    let insertPoint = doc.querySelector('#properties-side-insert-pt');
                    if (insertPoint == null) {
                        insertPoint = doc.createElement('div');
                        insertPoint.id = 'properties-side-insert-pt';
                        sideBar.insertBefore(insertPoint, e);
                    }

                    let title = doc.createElement('h3');
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

    async function fetchInherits(doc, inherits) {
        for (e of doc.querySelectorAll('h2.inherit-fetch')) {
            let page;
            let url = e.querySelector('a').href;
            let place = e.nextElementSibling;
            try {
                page = await fetch(url);
                var parser = new DOMParser();
                page = parser.parseFromString(await page.text(), 'text/html');

                let baseEl = page.createElement('base');
                baseEl.setAttribute('href', url);
                page.head.append(baseEl);
            } catch (error) {
                place.innerText = error;
                continue;
            }
            e.classList.remove('inherit-fetch');

            let link = e.querySelector('a').outerHTML;
            inherits.push({
                link, page
            });

            await refactorDocument(page, inherits);

            place.remove();
            e.remove();
        }
    }

    function mergeInherits(inherits) {
        let insertPoint = this.document.getElementById("properties-insert-pt");
        inherits.forEach(function (e) {
            let parentProps = e.page.getElementById('properties');
            if (parentProps != null) {

                let title = this.document.createElement('h2');
                title.classList.add("small-section-header");

                title.innerHTML = e.link;
                let name = title.querySelector('a').innerText;
                title.id = 'properties-' + name;

                title.innerHTML = "Properties from " + e.link + '<a href="#properties-' + name + '" class="anchor">§</a></h2>';

                insertPoint.parentElement.insertBefore(title, insertPoint);
            }

            e.page.querySelectorAll('h2.small-section-header').forEach(function (e) {
                if (e.id.indexOf('properties-') !== -1) {
                    insertPoint.parentElement.insertBefore(e, insertPoint);
                }
            });
        });
    }
})();