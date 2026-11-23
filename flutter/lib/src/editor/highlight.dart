// Line-by-line lexers for LaTeX, Typst and BibTeX.
//
// Each line is lexed from the state the previous line ended in (math mode,
// block comment, …). After an edit only the changed lines are re-lexed, plus
// following lines until a line ends in the same state as before — so typing
// costs microseconds no matter how long the document is.

enum Lang { latex, typst, bib, plain }

Lang langFor(String path) {
  final p = path.toLowerCase();
  if (p.endsWith('.typ')) return Lang.typst;
  if (p.endsWith('.bib')) return Lang.bib;
  if (p.endsWith('.tex') || p.endsWith('.sty') || p.endsWith('.cls') || p.endsWith('.ltx') || p.endsWith('.dtx')) return Lang.latex;
  return Lang.plain;
}

enum Tok { command, keyword, atom, number, math, comment, bracket, string, heading, strong, emph }

/// Lexes `text` starting in `state`, appending `start, end, Tok.index`
/// triples to `out`. Returns the state at the end of the line.
typedef LineLexer = int Function(String text, int state, List<int> out);

LineLexer lexerFor(Lang lang) => switch (lang) {
  Lang.latex => _latex,
  Lang.typst => _typst,
  Lang.bib => _bib,
  Lang.plain => (_, s, _) => s,
};

bool _isLetter(int c) => (c >= 65 && c <= 90) || (c >= 97 && c <= 122) || c == 64;
bool _isIdent(int c) => (c >= 65 && c <= 90) || (c >= 97 && c <= 122) || (c >= 48 && c <= 57) || c == 45 || c == 95;
bool _isDigit(int c) => c >= 48 && c <= 57;

void _add(List<int> out, int a, int b, Tok k) {
  if (b > a) out.addAll([a, b, k.index]);
}

// ── LaTeX ─────────────────────────────────────────────────────────────────
// State: 0 text, 1 inline math ($ or \( ), 2 display math ($$ or \[ ),
// 3 verbatim-like environment (verbatim, lstlisting, minted).

const _latexKeywords = {
  'begin', 'end', 'documentclass', 'usepackage', 'RequirePackage', 'part', 'chapter', 'section', 'subsection', //
  'subsubsection', 'paragraph', 'input', 'include', 'newcommand', 'renewcommand', 'newenvironment', 'def', 'let',
};
const _verbatimEnvs = {'verbatim', 'verbatim*', 'lstlisting', 'minted', 'Verbatim', 'comment'};

int _latex(String t, int state, List<int> out) {
  final n = t.length;
  if (state == 3) {
    final e = t.indexOf(r'\end{');
    if (e < 0) {
      _add(out, 0, n, Tok.string);
      return 3;
    }
    _add(out, 0, e, Tok.string);
    // Lex the rest normally (the \end{…} itself).
    final rest = <int>[];
    final s = _latex(t.substring(e), 0, rest);
    for (var i = 0; i < rest.length; i += 3) {
      out.addAll([rest[i] + e, rest[i + 1] + e, rest[i + 2]]);
    }
    return s;
  }
  var math = state;
  var mathStart = 0;
  var i = 0;
  while (i < n) {
    final c = t.codeUnitAt(i);
    if (c == 37) {
      if (math > 0) _add(out, mathStart, i, Tok.math);
      _add(out, i, n, Tok.comment);
      return math;
    }
    if (c == 92 && i + 1 < n) {
      final d = t.codeUnitAt(i + 1);
      if (math == 0 && (d == 91 || d == 40)) {
        math = d == 91 ? 2 : 1;
        mathStart = i;
        i += 2;
        continue;
      }
      if (math > 0 && (d == 93 || d == 41)) {
        _add(out, mathStart, i + 2, Tok.math);
        math = 0;
        i += 2;
        continue;
      }
      var j = i + 1;
      if (_isLetter(d)) {
        while (j < n && _isLetter(t.codeUnitAt(j))) {
          j++;
        }
      } else {
        j++;
      }
      if (math > 0) {
        i = j;
        continue;
      }
      final name = t.substring(i + 1, j);
      _add(out, i, j, _latexKeywords.contains(name) ? Tok.keyword : Tok.command);
      i = j;
      if ((name == 'begin' || name == 'end') && i < n && t.codeUnitAt(i) == 123) {
        final close = t.indexOf('}', i);
        if (close > 0) {
          _add(out, i, i + 1, Tok.bracket);
          _add(out, i + 1, close, Tok.atom);
          _add(out, close, close + 1, Tok.bracket);
          final env = t.substring(i + 1, close);
          i = close + 1;
          if (name == 'begin' && _verbatimEnvs.contains(env)) {
            // Options on the same line stay normal; the body is verbatim.
            final rest = t.substring(i);
            final e = rest.indexOf(r'\end{');
            if (e < 0) return 3;
            _add(out, i, i + e, Tok.string);
            i += e;
          }
        }
      }
      continue;
    }
    if (c == 36) {
      final dbl = i + 1 < n && t.codeUnitAt(i + 1) == 36;
      if (math == 0) {
        math = dbl ? 2 : 1;
        mathStart = i;
        i += dbl ? 2 : 1;
      } else {
        i += (math == 2 && dbl) ? 2 : 1;
        _add(out, mathStart, i, Tok.math);
        math = 0;
      }
      continue;
    }
    if (math == 0 && (c == 123 || c == 125 || c == 91 || c == 93)) _add(out, i, i + 1, Tok.bracket);
    i++;
  }
  if (math > 0) _add(out, mathStart, n, Tok.math);
  return math;
}

// ── Typst ─────────────────────────────────────────────────────────────────
// State bits: 1 block comment, 2 math, 4 raw block (```), bits 8+ open
// brackets of an embedded code expression.

const _typstKeywords = {
  'let', 'set', 'show', 'import', 'include', 'if', 'else', 'for', 'while', 'in', 'return', 'context', //
  'break', 'continue', 'and', 'or', 'not', 'none', 'auto', 'true', 'false',
};
const _units = ['pt', 'mm', 'cm', 'in', 'em', 'fr', 'deg', 'rad', '%'];

int _typst(String t, int state, List<int> out) {
  final n = t.length;
  var comment = state & 1 != 0;
  var math = state & 2 != 0;
  var raw = state & 4 != 0;
  var depth = state >> 3;
  var kwLine = false;
  var i = 0;

  if (raw) {
    final e = t.indexOf('```');
    if (e < 0) {
      _add(out, 0, n, Tok.string);
      return state;
    }
    _add(out, 0, e + 3, Tok.string);
    raw = false;
    i = e + 3;
  }
  if (comment) {
    final e = t.indexOf('*/');
    if (e < 0) {
      _add(out, 0, n, Tok.comment);
      return state;
    }
    _add(out, 0, e + 2, Tok.comment);
    comment = false;
    i = e + 2;
  }
  var mathStart = math ? i : 0;

  // `= Heading` (markup only).
  if (!math && depth == 0 && i == 0) {
    var j = 0;
    while (j < n && (t.codeUnitAt(j) == 32 || t.codeUnitAt(j) == 9)) {
      j++;
    }
    if (j < n && t.codeUnitAt(j) == 61) {
      _add(out, j, n, Tok.heading);
      return 0;
    }
  }

  int pack() => (comment ? 1 : 0) | (math ? 2 : 0) | (raw ? 4 : 0) | (depth << 3);

  while (i < n) {
    final c = t.codeUnitAt(i);
    final code = depth > 0 || kwLine;
    if (math) {
      if (c == 92) {
        i += 2;
        continue;
      }
      if (c == 36) {
        _add(out, mathStart, i + 1, Tok.math);
        math = false;
      }
      i++;
      continue;
    }
    if (c == 47 && i + 1 < n && t.codeUnitAt(i + 1) == 47) {
      _add(out, i, n, Tok.comment);
      return pack();
    }
    if (c == 47 && i + 1 < n && t.codeUnitAt(i + 1) == 42) {
      final e = t.indexOf('*/', i + 2);
      if (e < 0) {
        _add(out, i, n, Tok.comment);
        comment = true;
        return pack();
      }
      _add(out, i, e + 2, Tok.comment);
      i = e + 2;
      continue;
    }
    if (c == 96 && t.startsWith('```', i)) {
      final e = t.indexOf('```', i + 3);
      if (e < 0) {
        _add(out, i, n, Tok.string);
        raw = true;
        return pack();
      }
      _add(out, i, e + 3, Tok.string);
      i = e + 3;
      continue;
    }
    if (c == 96) {
      final e = t.indexOf('`', i + 1);
      final end = e < 0 ? n : e + 1;
      _add(out, i, end, Tok.string);
      i = end;
      continue;
    }
    if (c == 92) {
      i += 2;
      continue;
    }
    if (c == 34 && code) {
      var j = i + 1;
      while (j < n && t.codeUnitAt(j) != 34) {
        if (t.codeUnitAt(j) == 92) j++;
        j++;
      }
      final end = j < n ? j + 1 : n;
      _add(out, i, end, Tok.string);
      i = end;
      continue;
    }
    if (c == 36 && !code) {
      math = true;
      mathStart = i;
      i++;
      continue;
    }
    if (c == 35 || (code && _isLetter(c))) {
      var j = c == 35 ? i + 1 : i;
      while (j < n && _isIdent(t.codeUnitAt(j))) {
        j++;
      }
      final name = t.substring(c == 35 ? i + 1 : i, j);
      if (name.isEmpty) {
        i++;
        continue;
      }
      final kw = _typstKeywords.contains(name);
      final call = j < n && (t.codeUnitAt(j) == 40 || t.codeUnitAt(j) == 91);
      _add(out, i, j, kw ? Tok.keyword : (call || c == 35 ? Tok.command : Tok.atom));
      if (c == 35 && kw) kwLine = true;
      if (j < n && t.codeUnitAt(j) == 40) {
        _add(out, j, j + 1, Tok.bracket);
        depth++;
        j++;
      }
      i = j;
      continue;
    }
    if (code) {
      if (c == 40 || c == 123) depth++;
      if ((c == 41 || c == 125) && depth > 0) depth--;
    }
    if (c == 64 || c == 60) {
      var j = i + 1;
      while (j < n && (_isIdent(t.codeUnitAt(j)) || t.codeUnitAt(j) == 58 || t.codeUnitAt(j) == 46)) {
        j++;
      }
      if (c == 60 && j < n && t.codeUnitAt(j) == 62 && j > i + 1) {
        _add(out, i, j + 1, Tok.atom);
        i = j + 1;
        continue;
      }
      if (c == 64 && j > i + 1) {
        _add(out, i, j, Tok.atom);
        i = j;
        continue;
      }
    }
    if (_isDigit(c) && (i == 0 || !_isIdent(t.codeUnitAt(i - 1)))) {
      var j = i;
      while (j < n && (_isDigit(t.codeUnitAt(j)) || t.codeUnitAt(j) == 46)) {
        j++;
      }
      final unit = _units.firstWhere((u) => t.startsWith(u, j), orElse: () => '');
      if (unit.isNotEmpty || code) {
        _add(out, i, j + unit.length, Tok.number);
        i = j + unit.length;
        continue;
      }
      i = j;
      continue;
    }
    if (!code && (c == 42 || c == 95)) {
      final e = t.indexOf(String.fromCharCode(c), i + 1);
      if (e > i + 1) {
        _add(out, i, e + 1, c == 42 ? Tok.strong : Tok.emph);
        i = e + 1;
        continue;
      }
    }
    if (c == 91 || c == 93 || c == 40 || c == 41 || c == 123 || c == 125) _add(out, i, i + 1, Tok.bracket);
    i++;
  }
  if (math) _add(out, mathStart, n, Tok.math);
  // An unclosed `(` of a one-line `#let`/`#set` continues on the next line.
  return pack();
}

// ── BibTeX ────────────────────────────────────────────────────────────────

final _bibEntry = RegExp(r'@\w+');
final _bibField = RegExp(r'^\s*(\w+)\s*=');

int _bib(String t, int state, List<int> out) {
  final c = t.indexOf('%');
  final code = c < 0 ? t : t.substring(0, c);
  final e = _bibEntry.firstMatch(code);
  if (e != null) _add(out, e.start, e.end, Tok.keyword);
  final f = _bibField.firstMatch(code);
  if (f != null) {
    final s = f.start + f.group(0)!.indexOf(f.group(1)!);
    _add(out, s, s + f.group(1)!.length, Tok.command);
  }
  if (c >= 0) _add(out, c, t.length, Tok.comment);
  return 0;
}
