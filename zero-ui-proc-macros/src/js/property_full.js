(function () {

let title = document.getElementsByTagName('h1')[0];
title.innerHTML = title.innerHTML.replace('Function', 'Property');

let fn = document.querySelector('pre.rust.fn');
let ffn = document.getElementById('ffn');
ffn.innerHTML = fn.innerHTML;

fn.innerText = fn.innerText
                .replace(/.*fn /s, '')
                .replace(/(?!<[^>]*)\((?![^<]*>)\s*/s, ': {\n    ')
                .replace(/(?!<[^>]*)\s*\)\s+->(?![^<]*>).*/s, '\n}');

let set = new Set();
for (let a of ffn.getElementsByTagName('a')) {
     if (!set.has(a.innerText)) {
        fn.innerHTML = fn.innerHTML.replaceAll(a.innerText, a.outerHTML);
        set.add(a.innerText)
    }
}

})()