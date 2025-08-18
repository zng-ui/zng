# Markdown

A paragraph, with **bold** text , *italic* text, and ***both***.

Lorem ipsum dolor sit amet, ei nam copiosae invidunt accusamus. Vidit dicat cu pri, sit magna vocibus ut. Cum eu assum primis voluptatum. Est dolorum urbanitas elaboraret no, nisl definitiones cu sit.

You can also link [web](https://httpbin.org), [file](../../image/res/zdenek-machacek-unsplash.jpg), a [title](#table "Table"), or a footnote[^1]. And finally you can `inline` code.

## Blocks

Sample of other blocks items:

### Lists

Unordered lists:

* Item 1

  A paragraph item.
* Item 2
* Item 3

Ordered lists:

1. Item 1
2. Item 2
3. Item 3

Task lists:

- [ ] Item 1
- [x] Item 2
- [ ] Item 3

Nested lists:

* Item 1
    * Item 1.1
        * Item 1.1.1
            * Item 1.1.1.1

Definition lists:

Term 1
: Definition of "Term 1".
: Second line.

Term 2
: Definition of *Term 2*.  

### Images

Image from local file:

![two hummingbirds, flying, looking at each other](../../image/res/zdenek-machacek-unsplash.jpg "Title text")

Image from the web:

![httpbin image](https://httpbin.org/image)

### Block Quote

A block quote:

> Lorem ipsum dolor sit amet, ei nam copiosae invidunt accusamus.
> Vidit dicat cu pri, sit magna vocibus ut. Cum eu assum primis voluptatum.

Text after quote.

> Outer quote. 
>> Nested quote.

### Code Block

A code block, with raw text:

```
Text

* Monospace.
* Manually formatted.
```

ANSI escape codes with lang `console` or `ansi`:

```console
&#x1b;[31mRED&#x1b;[0m normal text
&#x1b;[32;1mGREEN&amp;BOLD&#x1b;[0m normal text
&#x1b;[34mBLUE&#x1b;[0m normal text
```

### Rule

Before rule. 

-------------

After rule.

### Table

| Tables   |      Are      |  Cool |
|----------|:-------------:|------:|
| col 1 is |  left-aligned | $1600 |
| col 2 is |    centered   |   $12 |
| col 3 is | right-aligned |    $1 |

## HTML

Limited support for <b>inlined</b> HTML.

## Footnotes

Footnote declarations.

[^1]: My reference.
