// Simple slides without packages: every level-1 heading starts a new slide.
#set page(paper: "presentation-16-9", margin: 2cm, numbering: "1", number-align: right)
#set text(size: 22pt, lang: "en")
#show heading.where(level: 1): it => {
  pagebreak(weak: true)
  text(size: 28pt, fill: rgb("#1f2a5c"), it.body)
  v(0.5em)
}

#align(center + horizon)[
  #text(size: 36pt, weight: "bold")[Presentation Title]

  Your Name · #datetime.today().display("[day].[month].[year]")
]

= Outline
- Introduction
- Main part
- Summary

= Introduction
- First point
- Second point
