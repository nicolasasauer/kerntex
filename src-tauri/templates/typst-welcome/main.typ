// Welcome to Typst in KernTeX!
//
// Typst is a modern alternative to LaTeX: simpler markup, fast compiling and
// clear error messages. This project is a short tour — change anything, press
// Ctrl+S and watch the PDF on the right. Reopen it any time via the command
// palette (Ctrl+K) → "Open Typst example project".

#set document(title: "Welcome to Typst", author: "KernTeX")
#set page(paper: "a4", margin: 2.5cm, numbering: "1")
#set text(size: 11pt, lang: "en")
#set par(justify: true)
#set heading(numbering: "1.")
#set math.equation(numbering: "(1)")
#show link: set text(fill: rgb("#1f4fa0"))

#align(center)[
  #text(size: 20pt, weight: "bold")[Welcome to Typst]

  A short tour, written in KernTeX
]

#block(inset: (x: 2em))[
  This document is your playground. It shows the most common things you will
  do in Typst: structure, formatting, lists, mathematics, tables, figures,
  cross-references, a bibliography, drawings and code. Read the source on the
  left side by side with this PDF.
]

#outline()

= Structure <sec:structure>
Headings start with `=`, `==`, `===`. The outline above is generated
automatically.

== A subsection
Paragraphs are separated by an empty line in the source.

Like this one.

= Formatting
You can write *bold*, _italic_, #underline[underlined], `monospace`,
#smallcaps[Small Caps] and #text(fill: rgb("#1f4fa0"))[coloured] text. Sizes:
#text(size: 9pt)[small], #text(size: 14pt)[large]. Quotes are written "like
this", dashes as en -- dash and em --- dash.

= Lists
- Bullet points start with `-`
- and can be
  + nested and
  + numbered with `+`.

/ Term lists: pair a term with an explanation.

= Mathematics <sec:math>
Inline math goes between dollar signs: $a^2 + b^2 = c^2$. Put spaces inside the
dollars for a displayed, numbered equation:
$ integral_(-oo)^oo e^(-x^2) dif x = sqrt(pi) $ <eq:gauss>

Several aligned lines use `&` and `\`:
$ f(x) &= (x + 1)^2 \
       &= x^2 + 2x + 1 $

Matrices and more:
$ A = mat(1, 2; 3, 4), quad sum_(k=1)^n k = (n(n+1)) / 2, quad
  forall epsilon > 0 thick exists delta > 0 $

= Tables
#figure(
  table(
    columns: 3,
    stroke: none,
    table.hline(),
    table.header([*Engine*], [*Format*], [*Comes with*]),
    table.hline(stroke: 0.5pt),
    [Typst], [`.typ`], [KernTeX (built in)],
    [Tectonic], [`.tex`], [KernTeX (built in)],
    [pdfLaTeX], [`.tex`], [TeX Live, MiKTeX],
    table.hline(),
  ),
  caption: [Engines KernTeX can use.],
) <tab:engines>

= Figures
Images are included with `image(...)` (PNG, JPG, SVG and PDF work).
@fig:example lives in the `images` folder of this project.

#figure(
  image("images/example.png", width: 45%),
  caption: [An example image.],
) <fig:example>

= Cross-references
Put a `<label>` after a heading, figure or equation and refer to it with `@`:
@sec:math contains @eq:gauss, and @tab:engines lists the engines. In the editor,
type `@` and KernTeX suggests the labels of the project.

= Bibliography
Typst reads ordinary BibTeX files. Cite with `@key`: LaTeX was created by
Lamport @lamport1994, building on Knuth's TeX @knuth1984. Tips on writing are
in @strunk.

= Drawings
Typst has shapes built in:
#align(center, stack(
  dir: ltr,
  spacing: 1em,
  rect(width: 3cm, height: 1.5cm, radius: 4pt, fill: rgb("#1f2a5c"), align(center + horizon, text(fill: white)[Typst])),
  circle(radius: 0.75cm, fill: rgb("#ffc94d")),
  polygon.regular(vertices: 6, size: 1.6cm, fill: rgb("#7b93f6")),
))

= Code
```python
def greet(name):
    # Code blocks are highlighted automatically.
    return f"Hello, {name}!"
```

= Footnotes and links
Footnotes are numbered automatically.#footnote[Like this one.] Links work too:
#link("https://typst.app/docs")[the Typst documentation] or
#link("https://github.com/nicolasasauer/kerntex")[KernTeX on GitHub].

= Scripting
Typst is also a small programming language:
#let fruits = ("apple", "banana", "cherry")
#for (i, f) in fruits.enumerate() [#(i + 1). #f #h(1em)]

= KernTeX tips
/ Ctrl+S: Save and compile.
/ Ctrl+K: Command palette — every action, searchable.
/ Problems panel: Click an error to jump to it; Typst marks the exact spot.
/ Packages: `#import "@preview/name:version"` — downloaded once, then cached.

#bibliography("references.bib")
