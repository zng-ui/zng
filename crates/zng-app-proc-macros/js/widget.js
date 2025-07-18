(function () {
    addEventListener("DOMContentLoaded", async function () {
        let inherits = [];
        await refactorDocument(this.document, new Set(), inherits, new Set());
        mergeInherits(inherits);
    });

    // * `doc` - The document, can be this.document or a fetched doc.
    // * `propertiesSet` - Tracks property name overrides.
    // * `inherits` - Array of `{ link, page }`, link is HTML str of the parent link.
    // * `fetchUrls` - Set of fetch URLs, used to avoid infinite recursion.
    async function refactorDocument(doc, propertiesSet, inherits, fetchUrls) {
        refactorSections(doc, propertiesSet, fetchUrls);
        refactorSidebar(doc, propertiesSet);
        // fetch inherit docs and refactorDocument
        await fetchInherits(doc, propertiesSet, inherits, fetchUrls);
    }

    function refactorSections(doc, propertiesSet, fetchUrls) {
        refactorProperties(doc, propertiesSet, 'implementations');
        doc.querySelectorAll('h2').forEach(function (d) {
            if (d.id.startsWith('deref-methods')) {
                let skipFirst = true;
                // details/summary/h2(d)
                let insertPoint = d.parentElement.parentElement.nextElementSibling;
                d.querySelectorAll('a.struct').forEach(function (a) {
                    if (skipFirst) {
                        // first's items already included in HTML
                        skipFirst = false;
                        return;
                    }

                    if (fetchUrls.has(a.href)) {
                        return;
                    }
                    fetchUrls.add(a.href);

                    // unloaded mixin parent

                    let title = doc.createElement('h2');
                    title.innerHTML = 'Inherits from ' + a.outerHTML;
                    title.classList.add('inherit-fetch');
                    let summary = doc.createElement("summary");
                    summary.appendChild(title);
                    let details = doc.createElement("details");
                    details.appendChild(summary);

                    let section = doc.createElement('div');
                    section.innerText = 'Loading...';
                    details.appendChild(section);

                    insertPoint.parentElement.insertBefore(details, insertPoint);

                    insertPoint = details.nextElementSibling;
                    a.remove();
                });
                refactorProperties(doc, propertiesSet, d.id);
            }
        });
    }

    function refactorProperties(doc, propertiesSet, sectionId) {
        let implementations = doc.getElementById(sectionId);
        if (implementations == null) {
            return;
        }

        let implementationsList = implementations.nextSibling;
        if (implementations.parentElement.nodeName == 'SUMMARY') {
            implementationsList = implementations.parentElement.nextSibling;
        }

        let isDeref = implementations.innerHTML.indexOf("Methods from") !== -1;
        let derefFrom = '';
        let derefIdPrefix = '';
        if (isDeref) {
            let mixCut = sectionId.indexOf('%3C');
            if (mixCut !== -1) {
                sectionId = sectionId.substring(0, mixCut);
            }
            derefIdPrefix = sectionId.replace('deref-methods', '');
            let structA = implementations.querySelector('span a:nth-of-type(2)').outerHTML;
            derefFrom = ' from ' + structA;

            implementations.id = sectionId;
            implementations.innerHTML = "Methods from " + structA + '<a href="#' + sectionId + '" class="anchor">§</a>';
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
            let t = e.querySelector("strong[data-tag='P'],strong[data-tag='c']");
            if (t != null) {
                let mtdAnchor = e.querySelector('a.fn');
                let propertyName = mtdAnchor.innerText;
                if (propertiesSet.has(propertyName)) {
                    // override
                    e.remove();
                } else {
                    propertiesSet.add(propertyName);
                    t.remove();
                    let mtdSignature = mtdAnchor.parentElement;
                    // same syntax as `widget_impl!`
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

    function refactorSidebar(doc, propertiesSet) {
        let sideBar = doc.querySelector('div.sidebar-elems section');
        let repeats = new Set();

        sideBar.querySelectorAll('h3').forEach(function (e) {
            if (e.innerText == "Methods" || e.innerText.indexOf("Methods from") !== -1) {
                e.firstChild.innerText = e.firstChild.innerText.replace('Deref<Target=', '').replace('>', '');
                let mixGenericsCut = e.firstChild.innerText.indexOf('<');
                if (mixGenericsCut !== -1) {
                    e.firstChild.innerText = e.firstChild.innerText.substring(0, mixGenericsCut);
                    mixGenericsCut = e.firstChild.href.indexOf('%3C');
                    if (mixGenericsCut !== -1) {
                        e.firstChild.href = e.firstChild.href.substring(0, mixGenericsCut);
                    }
                }

                let mtdList = e.nextSibling;
                let propList = doc.createElement('ul');
                mtdList.querySelectorAll('a').forEach(function (a) {
                    if (propertiesSet.has(a.innerText)) {
                        if (repeats.has(a.innerText)) {
                            a.remove();
                        } else {
                            repeats.add(a.innerText);
                            propList.appendChild(a.parentElement);
                        }

                    }
                });

                // insert property sections before first impl sections.
                // used here and by `mergeInherits`
                let insertPoint = doc.querySelector('#properties-side-insert-pt');
                if (insertPoint == null) {
                    insertPoint = doc.createElement('div');
                    insertPoint.id = 'properties-side-insert-pt';
                    sideBar.insertBefore(insertPoint, e);
                }

                if (propList.hasChildNodes()) {
                    propList.classList.add('block');

                    let title = doc.createElement('h3');
                    let mtdsFrom = e.querySelector('a').href.indexOf('#deref-methods-');
                    if (mtdsFrom !== -1) {
                        let parentName = e.innerText.substring("Methods from ".length);
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

    async function fetchInherits(doc, propertiesSet, inherits, fetchUrls) {
        for (e of doc.querySelectorAll('h2.inherit-fetch')) {
            let page;
            let url = e.querySelector('a').href;
            let place = e.parentElement.parentElement;
            try {
                page = await fetch(url, { redirect: 'follow' });
                if (!page.ok) {
                    throw page.statusText;
                }
                var parser = new DOMParser();
                if (page.redirected) {
                    url = page.url;
                }
                page = parser.parseFromString(await page.text(), 'text/html');

                let refresh = page.head.querySelector("meta[http-equiv='refresh']");
                if (refresh != null && refresh.content.startsWith('0;URL=')) {
                    url = refresh.content.replace('0;URL=', '');
                    page = await fetch(url, { redirect: 'follow' });
                    if (!page.ok) {
                        throw page.statusText;
                    }
                    url = page.url;
                    var parser = new DOMParser();
                    page = parser.parseFromString(await page.text(), 'text/html');
                }
            } catch (error) {
                console.error("error fetching '" + url + "', " + error);
                place.querySelector("div") = error;
                continue;
            }
            e.classList.remove('inherit-fetch');

            let link = e.querySelector('a').outerHTML;
            inherits.push({
                link, page, url
            });

            await refactorDocument(page, propertiesSet, inherits, fetchUrls);

            place.remove();
        }
    }

    function mergeInherits(inherits) {
        let insertPoint = this.document.getElementById("properties-insert-pt");
        let sideInsertPoint = this.document.getElementById("properties-side-insert-pt");
        let insertedDocs = [];
        inherits.forEach(function (e) {
            // merge properties
            let side = e.page.querySelector('.sidebar-elems');
            let parentProps = e.page.getElementById('properties');
            if (parentProps != null) {

                let title = this.document.createElement('h2');
                title.classList.add("small-section-header");

                title.innerHTML = e.link;
                let name = title.querySelector('a').innerText;
                title.id = 'properties-' + name;
                title.innerHTML = "Properties from " + e.link + '<a href="#properties-' + name + '" class="anchor">§</a></h2>';

                parentProps.nextElementSibling.id = title.id + "-list";
                insertedDocs.push({
                    url: e.url,
                    el: parentProps.nextElementSibling,
                });

                insertPoint.parentElement.insertBefore(title, insertPoint);
                insertPoint.parentElement.insertBefore(parentProps.nextElementSibling, insertPoint);

                let sideTitle = side.querySelector('h3');
                let sideTitleA = sideTitle.querySelector('a');
                sideTitleA.innerText = title.innerText;
                sideTitleA.href = '#properties-' + name;
                let sideList = side.querySelector('ul');

                sideInsertPoint.parentElement.insertBefore(sideTitle, sideInsertPoint);
                sideInsertPoint.parentElement.insertBefore(sideList, sideInsertPoint);
            }
            e.page.querySelectorAll('h2.small-section-header').forEach(function (e) {
                if (e.id.indexOf('properties-') !== -1) {
                    let list = e.nextElementSibling;
                    insertPoint.parentElement.insertBefore(e, insertPoint);
                    insertPoint.parentElement.insertBefore(list, insertPoint);
                }
            });
            side.querySelectorAll('h3').forEach(function (e) {
                if (e.querySelector('a').href.indexOf('#properties-') !== -1) {
                    let sideList = e.nextElementSibling;
                    sideInsertPoint.parentElement.insertBefore(e, sideInsertPoint);
                    sideInsertPoint.parentElement.insertBefore(sideList, sideInsertPoint);
                }
            });

            // merge methods
            let methodsInsertPoint = null;
            this.document.querySelectorAll('h2.small-section-header').forEach(function (e) {
                if (e.id.indexOf('deref-methods-') !== -1) {
                    methodsInsertPoint = e;
                }
            });
            if (methodsInsertPoint === null) {
                let impls = this.document.getElementById('implementations');
                if (impls !== null) {
                    methodsInsertPoint = impls.nextElementSibling.nextElementSibling;
                } else {
                    methodsInsertPoint = insertPoint.nextElementSibling;
                }
            } else {
                methodsInsertPoint = methodsInsertPoint.nextElementSibling.nextElementSibling;
            }
            let methodsSideInsertPoint = null;
            sideInsertPoint.parentElement.querySelectorAll('h3').forEach(function (e) {
                let href = e.querySelector('a').href;
                if (href.indexOf('#deref-methods-') !== -1 || href.indexOf('#implementations') !== -1) {
                    methodsSideInsertPoint = e;
                }
            });
            if (methodsSideInsertPoint === null) {
                methodsSideInsertPoint = sideInsertPoint.nextElementSibling;
            } else {
                methodsSideInsertPoint = methodsSideInsertPoint.nextElementSibling.nextElementSibling;
            }

            let parentMethods = e.page.getElementById('implementations');
            if (parentMethods != null) {
                let title = this.document.createElement('h2');
                title.classList.add("small-section-header");

                title.innerHTML = e.link;
                let name = title.querySelector('a').innerText;
                title.id = 'deref-methods-' + name;
                title.innerHTML = "Methods from " + e.link + '<a href="#deref-methods-' + name + '" class="anchor">§</a></h2>';

                let mtdList = this.document.createElement('div');
                mtdList.classList.add("impl-items");

                let mtdNames = new Set();

                parentMethods.nextElementSibling.querySelectorAll('details.method-toggle').forEach(function (e) {
                    let mtd = e.querySelector('h4').innerText;
                    if (mtd.indexOf('&self') !== -1 || mtd.indexOf('&mut self') !== -1) {
                        mtdNames.add(e.querySelector('h4 a').innerText);
                        mtdList.appendChild(e);
                    }
                });
                if (mtdNames.size > 0) {
                    methodsInsertPoint.parentElement.insertBefore(title, methodsInsertPoint);
                    methodsInsertPoint.parentElement.insertBefore(mtdList, methodsInsertPoint);

                    insertedDocs.push({
                        url: e.url,
                        el: mtdList,
                    });

                    let sideList = side.querySelector('a[href="#implementations"]').parentElement.nextElementSibling;
                    sideList.querySelectorAll('a').forEach(function (a) {
                        if (!mtdNames.has(a.innerText)) {
                            a.parentElement.remove();
                        }
                    });

                    let sideTitle = this.document.createElement('h3');
                    sideTitle.innerHTML = '<a href="#' + title.id + '">Methods from ' + name + '</a>';
                    methodsSideInsertPoint.parentElement.insertBefore(sideTitle, methodsSideInsertPoint);
                    methodsSideInsertPoint.parentElement.insertBefore(sideList, methodsSideInsertPoint);
                }
            }

            e.page.querySelectorAll('h2.small-section-header').forEach(function (e) {
                if (e.id.indexOf('deref-methods-') !== -1) {
                    let list = e.nextElementSibling;
                    methodsInsertPoint.parentElement.insertBefore(e, methodsInsertPoint);
                    methodsInsertPoint.parentElement.insertBefore(list, methodsInsertPoint);
                }
            });

            side.querySelectorAll('h3').forEach(function (e) {
                if (e.querySelector('a').href.indexOf('#deref-methods-') !== -1) {
                    let sideList = e.nextElementSibling;
                    methodsSideInsertPoint.parentElement.insertBefore(e, methodsSideInsertPoint);
                    methodsSideInsertPoint.parentElement.insertBefore(sideList, methodsSideInsertPoint);
                }
            });
        });

        insertedDocs.forEach(function (e) {
            e.el.querySelectorAll('a').forEach(function (a) {
                let href_str = a.getAttribute('href');
                if (!href_str.startsWith('#') || this.document.getElementById(href_str.slice(1)) === null) {
                    let url = new URL(href_str, e.url);
                    a.setAttribute('href', url.href);
                }
            });
        });

        document.querySelectorAll("div[id^='properties-']").forEach(function (e) {
            if (e.id.startsWith("properties-") && e.id.endsWith("-list") && e.querySelector('a') === null) {
                e.previousSibling.remove();
                e.remove();
            }
        });
    }
})();