#set document(title: "Thesis Title", author: "Your Name")
#set page(paper: "a4", margin: 2.5cm, numbering: "1")
#set text(size: 11pt, lang: "en")
#set par(justify: true)
#set heading(numbering: "1.1")
#show heading.where(level: 1): it => { pagebreak(weak: true); it }
#set math.equation(numbering: "(1)")

#align(center + horizon)[
  #text(size: 24pt, weight: "bold")[Thesis Title]

  #v(1em)
  Your Name

  #datetime.today().display("[month repr:long] [year]")
]

#outline()

= Introduction <ch:intro>
Motivation, research question and structure of this work.

= Background <ch:background>
Related work and fundamentals.

= Method <ch:method>

= Results <ch:results>

= Conclusion <ch:conclusion>
