#set page(paper: "a4", margin: (x: 2.5cm, y: 2cm))
#set text(size: 11pt, lang: "en")
#set par(justify: true)

#align(right)[
  Your Name \
  Your Street 1 \
  12345 Your City
]

#v(1cm)
Recipient Name \
Their Street 2 \
54321 Their City

#v(1cm)
#align(right)[#datetime.today().display("[day] [month repr:long] [year]")]

*Subject of the letter*

Dear Sir or Madam,

Write your letter here.

Kind regards,

#v(1.5cm)
Your Name
