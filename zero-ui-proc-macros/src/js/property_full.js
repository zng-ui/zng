(function () {

let title = document.getElementsByTagName('h1')[0];
title.innerHTML = title.innerHTML.replace('Function', 'Property');

let fn = document.querySelector('pre.rust.fn');
let ffn = document.getElementById('ffn');
ffn.innerHTML = fn.innerHTML;

fn.innerHTML = fn.innerHTML
    .replace(/.*fn /, '')
    .replace(/(?!<[^>]*)\((<br>|&nbsp;|\s)*(?![^<]*>)/, ': {\n    ')// first fn open `(` not inside `<generics>`
    .replace(/(?:<br>)?(?!<[^>]*)\) -&gt;(?![^<]*>).*/, '\n};');// fn close and return type `) -> *`

})()