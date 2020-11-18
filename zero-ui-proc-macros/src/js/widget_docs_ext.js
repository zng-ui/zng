document.addEventListener('DOMContentLoaded', function() {
    var ul = document.querySelector('div.block.items ul');
    if (ul === null) {
        var sidebar_elems = document.querySelector('div.sidebar-elems');
        var block_items = document.createElement('div');
        block_items.className = 'block items';
        ul = document.createElement('ul');
        block_items.append(ul);
        sidebar_elems.prepend(block_items);
    }
    prepend_item('other-properties', 'Other properties', ul);
    prepend_item('state-properties', 'State properties', ul);
    prepend_item('provided-properties', 'Provided properties', ul);
    prepend_item('required-properties', 'Required properties', ul);
    var any_help_request = document.querySelector('span.load-property-help');
    if (any_help_request !== null) {
        var frame = document.createElement('iframe');
        frame.id = 'doc_helper_frame';
        frame.src = 'doc_helper/index.html';
        frame.style = 'display:none';
        document.body.append(frame);
    }
});
function prepend_item(id, text, ul) {
    if (document.getElementById(id) !== null) {
        var li = document.createElement('li');
        li.innerHTML = `<a href='#${id}'>${text}</a>`;
        ul.prepend(li);
    }
}
window.addEventListener('message', function(e) {
    document.getElementById('doc_helper_frame').remove();
    var requests = document.querySelectorAll('span.load-property-help');
    var parse = document.createElement('div');
    parse.innerHTML = e.data;
    var help = parse.childNodes[0];
    var property_help = {};
    for (row of help.rows) {
        console.log(row);
        var property = row.querySelector('a.mod').innerHTML;
        var help  = row.querySelector('p');
        property_help[property] = help;
    }
    var requests = document.querySelectorAll('span.load-property-help');
    requests.forEach(req => {
        var property = req.getAttribute('data-property');
        var value = property_help[property];
        if (value !== null) {
            req.parentElement.replaceWith(value);
        } else {
            req.parentElement.replaceWith("")
        }
    });
});