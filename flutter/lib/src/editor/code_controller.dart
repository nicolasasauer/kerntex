// A TextEditingController that paints LaTeX / Typst syntax colours and
// diagnostic underlines. Tokenising is a single linear pass, cached per text,
// so typing re-scans once per change and repaints reuse the result.

import 'package:material_ui/material_ui.dart';

import '../theme.dart';

enum Lang { latex, typst, bib, plain }

Lang langFor(String path) {
  final p = path.toLowerCase();
  if (p.endsWith('.typ')) return Lang.typst;
  if (p.endsWith('.bib')) return Lang.bib;
  if (p.endsWith('.tex') || p.endsWith('.sty') || p.endsWith('.cls') || p.endsWith('.ltx')) return Lang.latex;
  return Lang.plain;
}

enum Tok { command, keyword, atom, number, math, comment, bracket, string, heading, strong, emph }

/// A diagnostic range in this file, 0-based offsets.
class Mark {
  const Mark(this.start, this.end, this.severity);
  final int start, end;
  final String severity;
}

class CodeController extends TextEditingController {
  CodeController({required String text, required this.lang}) : super(text: text);

  final Lang lang;
  List<Mark> marks = const [];

  String? _scanned;
  List<int> _tokens = const []; // flat triples: start, end, Tok.index
  List<int> _lineStarts = const [0];
  int _maxLine = 0;

  /// Length of the longest line, in characters (sizes the scroll width).
  int get maxLineLength {
    _scan();
    return _maxLine;
  }

  List<int> get lineStarts {
    _scan();
    return _lineStarts;
  }

  int get lineCount => lineStarts.length;

  /// 0-based line of a text offset.
  int lineOf(int offset) {
    final ls = lineStarts;
    var lo = 0, hi = ls.length - 1;
    while (lo < hi) {
      final mid = (lo + hi + 1) >> 1;
      if (ls[mid] <= offset) {
        lo = mid;
      } else {
        hi = mid - 1;
      }
    }
    return lo;
  }

  /// Offset of 1-based `line` / `column` (clamped into the text).
  int offsetOf(int line, [int column = 1]) {
    final ls = lineStarts;
    final l = (line - 1).clamp(0, ls.length - 1);
    final end = l + 1 < ls.length ? ls[l + 1] - 1 : text.length;
    return (ls[l] + column - 1).clamp(ls[l], end);
  }

  /// Sets diagnostics (1-based line/column, as the core reports them).
  void setDiagnostics(List<Map<String, dynamic>> diags) {
    final out = <Mark>[];
    for (final d in diags) {
      final line = d['line'] as int?;
      if (line == null) continue;
      final col = d['column'] as int?;
      int start, end;
      if (col != null) {
        start = offsetOf(line, col);
        end = offsetOf((d['endLine'] as int?) ?? line, (d['endColumn'] as int?) ?? col + 1);
        if (end <= start) end = (start + 1).clamp(0, text.length);
      } else {
        // TeX only knows the line: underline its content, without indentation.
        start = offsetOf(line);
        end = offsetOf(line, 1 << 30);
        while (start < end && (text.codeUnitAt(start) == 32 || text.codeUnitAt(start) == 9)) {
          start++;
        }
      }
      if (end > start) out.add(Mark(start, end, d['severity'] as String? ?? 'error'));
    }
    out.sort((a, b) => a.start.compareTo(b.start));
    marks = out;
    notifyListeners();
  }

  @override
  set value(TextEditingValue v) {
    // Diagnostics refer to the text that was compiled; drop them once it changes.
    if (v.text != text && marks.isNotEmpty) marks = const [];
    super.value = v;
  }

  void _scan() {
    final t = text;
    if (identical(t, _scanned)) return;
    _scanned = t;
    final ls = <int>[0];
    var maxLine = 0;
    for (var i = 0; i < t.length; i++) {
      if (t.codeUnitAt(i) == 10) {
        if (i - ls.last > maxLine) maxLine = i - ls.last;
        ls.add(i + 1);
      }
    }
    if (t.length - ls.last > maxLine) maxLine = t.length - ls.last;
    _lineStarts = ls;
    _maxLine = maxLine;
    _tokens = switch (lang) {
      Lang.latex => _latex(t),
      Lang.typst => _typst(t),
      Lang.bib => _bib(t),
      Lang.plain => const [],
    };
  }

  @override
  TextSpan buildTextSpan({required BuildContext context, TextStyle? style, required bool withComposing}) {
    if (withComposing && value.isComposingRangeValid) {
      return super.buildTextSpan(context: context, style: style, withComposing: withComposing);
    }
    _scan();
    final k = KT.of(context);
    final styles = <TextStyle>[
      TextStyle(color: k.synCommand),
      TextStyle(color: k.synKeyword, fontWeight: FontWeight.w600),
      TextStyle(color: k.synAtom),
      TextStyle(color: k.synNumber),
      TextStyle(color: k.synMath),
      TextStyle(color: k.synComment, fontStyle: FontStyle.italic),
      TextStyle(color: k.synBracket),
      TextStyle(color: k.synString),
      TextStyle(color: k.accent, fontWeight: FontWeight.w700),
      const TextStyle(fontWeight: FontWeight.w700),
      const TextStyle(fontStyle: FontStyle.italic),
    ];
    final t = text;
    final spans = <InlineSpan>[];

    // Boundaries from tokens and marks, walked together in one pass.
    final cuts = <int>{0, t.length};
    for (var i = 0; i < _tokens.length; i += 3) {
      cuts
        ..add(_tokens[i])
        ..add(_tokens[i + 1]);
    }
    for (final m in marks) {
      cuts
        ..add(m.start.clamp(0, t.length))
        ..add(m.end.clamp(0, t.length));
    }
    final points = cuts.toList()..sort();
    var ti = 0, mi = 0;
    for (var p = 0; p + 1 < points.length; p++) {
      final a = points[p], b = points[p + 1];
      if (a == b) continue;
      while (ti < _tokens.length && _tokens[ti + 1] <= a) {
        ti += 3;
      }
      TextStyle? s = (ti < _tokens.length && _tokens[ti] <= a) ? styles[_tokens[ti + 2]] : null;
      while (mi < marks.length && marks[mi].end <= a) {
        mi++;
      }
      for (var j = mi; j < marks.length && marks[j].start <= a; j++) {
        if (marks[j].end > a) {
          final c = switch (marks[j].severity) {
            'warning' => k.warning,
            'info' => k.info,
            _ => k.danger,
          };
          s = (s ?? const TextStyle()).copyWith(
            decoration: TextDecoration.underline,
            decorationStyle: TextDecorationStyle.wavy,
            decorationColor: c,
            backgroundColor: marks[j].severity == 'error' ? k.dangerSoft.withValues(alpha: 0.6) : null,
          );
          break;
        }
      }
      spans.add(TextSpan(text: t.substring(a, b), style: s));
    }
    return TextSpan(style: style, children: spans);
  }
}

bool _isLetter(int c) => (c >= 65 && c <= 90) || (c >= 97 && c <= 122) || c == 64; // @ for .sty files
bool _isIdent(int c) => _isLetter(c) || (c >= 48 && c <= 57) || c == 45 || c == 95;
bool _isDigit(int c) => c >= 48 && c <= 57;

const _latexKeywords = {
  'begin', 'end', 'documentclass', 'usepackage', 'RequirePackage', 'part', 'chapter', 'section', 'subsection', //
  'subsubsection', 'paragraph', 'input', 'include', 'newcommand', 'renewcommand', 'newenvironment', 'def', 'let',
};

List<int> _latex(String t) {
  final out = <int>[];
  var math = 0; // 0 none, 1 $, 2 $$ or \[ or \(
  var mathStart = 0;
  void add(int a, int b, Tok k) {
    if (b > a) out.addAll([a, b, k.index]);
  }

  var i = 0;
  final n = t.length;
  while (i < n) {
    final c = t.codeUnitAt(i);
    if (c == 37) {
      // % comment
      final e = t.indexOf('\n', i);
      final end = e < 0 ? n : e;
      if (math > 0) add(mathStart, i, Tok.math);
      add(i, end, Tok.comment);
      i = end;
      if (math > 0) mathStart = i;
      continue;
    }
    if (c == 92 && i + 1 < n) {
      // \command
      final d = t.codeUnitAt(i + 1);
      if (math == 0 && (d == 91 || d == 40)) {
        math = 2;
        mathStart = i;
        i += 2;
        continue;
      }
      if (math == 2 && (d == 93 || d == 41)) {
        add(mathStart, i + 2, Tok.math);
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
      final kw = _latexKeywords.contains(name);
      add(i, j, kw ? Tok.keyword : Tok.command);
      i = j;
      if ((name == 'begin' || name == 'end') && i < n && t.codeUnitAt(i) == 123) {
        final close = t.indexOf('}', i);
        if (close > 0 && t.lastIndexOf('\n', close) < i) {
          add(i, i + 1, Tok.bracket);
          add(i + 1, close, Tok.atom);
          add(close, close + 1, Tok.bracket);
          i = close + 1;
        }
      }
      continue;
    }
    if (c == 36) {
      // $ / $$
      final dbl = i + 1 < n && t.codeUnitAt(i + 1) == 36;
      if (math == 0) {
        math = dbl ? 2 : 1;
        mathStart = i;
        i += dbl ? 2 : 1;
      } else {
        i += (math == 2 && dbl) ? 2 : 1;
        add(mathStart, i, Tok.math);
        math = 0;
      }
      continue;
    }
    if (math == 0 && (c == 123 || c == 125 || c == 91 || c == 93)) add(i, i + 1, Tok.bracket);
    i++;
  }
  if (math > 0) add(mathStart, n, Tok.math);
  return out;
}

const _typstKeywords = {
  'let', 'set', 'show', 'import', 'include', 'if', 'else', 'for', 'while', 'in', 'return', 'context', //
  'break', 'continue', 'and', 'or', 'not', 'none', 'auto', 'true', 'false',
};
const _units = ['pt', 'mm', 'cm', 'in', 'em', 'fr', 'deg', 'rad', '%'];

List<int> _typst(String t) {
  final out = <int>[];
  void add(int a, int b, Tok k) {
    if (b > a) out.addAll([a, b, k.index]);
  }

  final n = t.length;
  var i = 0;
  var lineStart = true;
  var depth = 0; // open ( / { of an embedded code expression after `#`
  var kwLine = false; // `#let …` / `#set …`: code until the end of the line
  while (i < n) {
    final c = t.codeUnitAt(i);
    final code = (depth > 0 || kwLine) ? 1 : 0;
    if (c == 10) {
      lineStart = true;
      kwLine = false;
      i++;
      continue;
    }
    if (lineStart && code == 0) {
      var j = i;
      while (j < n && (t.codeUnitAt(j) == 32 || t.codeUnitAt(j) == 9)) {
        j++;
      }
      if (j < n && t.codeUnitAt(j) == 61) {
        // = Heading
        final e = t.indexOf('\n', j);
        add(j, e < 0 ? n : e, Tok.heading);
        i = e < 0 ? n : e;
        continue;
      }
    }
    lineStart = false;
    if (c == 47 && i + 1 < n && t.codeUnitAt(i + 1) == 47) {
      final e = t.indexOf('\n', i);
      add(i, e < 0 ? n : e, Tok.comment);
      i = e < 0 ? n : e;
      continue;
    }
    if (c == 47 && i + 1 < n && t.codeUnitAt(i + 1) == 42) {
      final e = t.indexOf('*/', i + 2);
      final end = e < 0 ? n : e + 2;
      add(i, end, Tok.comment);
      i = end;
      continue;
    }
    if (c == 92 && i + 1 < n) {
      i += 2; // escape
      continue;
    }
    if (c == 34 && code > 0) {
      var j = i + 1;
      while (j < n && t.codeUnitAt(j) != 34 && t.codeUnitAt(j) != 10) {
        if (t.codeUnitAt(j) == 92) j++;
        j++;
      }
      add(i, (j + 1).clamp(0, n), Tok.string);
      i = j + 1;
      continue;
    }
    if (c == 36 && code == 0) {
      var j = i + 1;
      while (j < n && t.codeUnitAt(j) != 36) {
        if (t.codeUnitAt(j) == 92) j++;
        j++;
      }
      add(i, (j + 1).clamp(0, n), Tok.math);
      i = j + 1;
      continue;
    }
    if (c == 35 || (code > 0 && _isLetter(c))) {
      // #ident / identifiers inside code
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
      add(i, j, kw ? Tok.keyword : (call || c == 35 ? Tok.command : Tok.atom));
      if (c == 35 && kw) kwLine = true;
      if (j < n && t.codeUnitAt(j) == 40) {
        add(j, j + 1, Tok.bracket);
        depth++;
        j++;
      }
      i = j;
      continue;
    }
    if (code > 0) {
      if (c == 40 || c == 123) depth++;
      if ((c == 41 || c == 125) && depth > 0) depth--;
    }
    if (c == 64 || c == 60) {
      // @ref / <label>
      var j = i + 1;
      while (j < n && (_isIdent(t.codeUnitAt(j)) || t.codeUnitAt(j) == 58 || t.codeUnitAt(j) == 46)) {
        j++;
      }
      if (c == 60 && j < n && t.codeUnitAt(j) == 62 && j > i + 1) {
        add(i, j + 1, Tok.atom);
        i = j + 1;
        continue;
      }
      if (c == 64 && j > i + 1) {
        add(i, j, Tok.atom);
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
      if (unit.isNotEmpty || code > 0) {
        add(i, j + unit.length, Tok.number);
        i = j + unit.length;
        continue;
      }
      i = j;
      continue;
    }
    if (code == 0 && (c == 42 || c == 95)) {
      // *strong* / _emph_ on one line
      final e = t.indexOf(String.fromCharCode(c), i + 1);
      final nl = t.indexOf('\n', i);
      if (e > i + 1 && (nl < 0 || e < nl)) {
        add(i, e + 1, c == 42 ? Tok.strong : Tok.emph);
        i = e + 1;
        continue;
      }
    }
    if (c == 91 || c == 93 || c == 40 || c == 41 || c == 123 || c == 125) add(i, i + 1, Tok.bracket);
    i++;
  }
  return out;
}

List<int> _bib(String t) {
  final out = <int>[];
  final entry = RegExp(r'@\w+');
  final field = RegExp(r'^\s*(\w+)\s*=', multiLine: true);
  final comment = RegExp(r'%[^\n]*');
  final all = <List<int>>[];
  for (final m in entry.allMatches(t)) {
    all.add([m.start, m.end, Tok.keyword.index]);
  }
  for (final m in field.allMatches(t)) {
    final s = m.start + m.group(0)!.indexOf(m.group(1)!);
    all.add([s, s + m.group(1)!.length, Tok.command.index]);
  }
  for (final m in comment.allMatches(t)) {
    all.add([m.start, m.end, Tok.comment.index]);
  }
  all.sort((a, b) => a[0].compareTo(b[0]));
  var last = 0;
  for (final r in all) {
    if (r[0] < last) continue;
    out.addAll(r);
    last = r[1];
  }
  return out;
}
