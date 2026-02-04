# analysis-string-parser

A library for parsing the typically "+" (but sometimes space) separated lemmas and
tags, coming from analysis.

For example:

```
$ echo "hei" | hfst-lookup -q /usr/share/giella/nob/analyser-gt-desc.hfstol
hei     hei+N+Fem+Sg+Indef      0,000000
hei     heie+V+Imp      0,000000
```

In this example, it focuses on parsing just the middle part, into a structure.
