// The editor's document: a list of lines with per-line highlighting, a
// selection, diagnostics that move with edits, and grouped undo/redo.
//
// A list of lines (rather than one big string) keeps every edit local: typing
// touches one line, inserting a line shifts a list of pointers. Nothing ever
// re-joins or re-scans the whole document while you type.

import 'dart:math' as math;

import 'package:flutter/foundation.dart';

import 'highlight.dart';

@immutable
class Pos implements Comparable<Pos> {
  const Pos(this.line, this.col);
  final int line, col;

  @override
  int compareTo(Pos o) => line != o.line ? line - o.line : col - o.col;
  bool operator <(Pos o) => compareTo(o) < 0;
  bool operator <=(Pos o) => compareTo(o) <= 0;
  bool operator >(Pos o) => compareTo(o) > 0;
  bool operator >=(Pos o) => compareTo(o) >= 0;

  @override
  bool operator ==(Object other) => other is Pos && other.line == line && other.col == col;
  @override
  int get hashCode => line * 1000003 + col;
  @override
  String toString() => '$line:$col';
}

@immutable
class Sel {
  const Sel(this.anchor, this.head);
  const Sel.at(Pos p) : anchor = p, head = p;
  final Pos anchor, head;
  bool get isEmpty => anchor == head;
  Pos get start => anchor < head ? anchor : head;
  Pos get end => anchor < head ? head : anchor;
  @override
  bool operator ==(Object other) => other is Sel && other.anchor == anchor && other.head == head;
  @override
  int get hashCode => Object.hash(anchor, head);
}

class Line {
  Line(this.text);
  String text;
  List<int> tokens = const [];
  int stateOut = -1;

  /// Layout cache owned by the view (a `ui.Paragraph`), with the generation
  /// it was built for.
  Object? layout;
  int layoutGen = -1;
}

/// A diagnostic range. `severity` is error / warning / info.
class Mark {
  Mark(this.start, this.end, this.severity, this.message);
  Pos start, end;
  final String severity, message;
}

class _Edit {
  _Edit(this.start, this.removed, this.inserted, this.before, this.after, this.revBefore, this.revAfter, this.kind);
  Pos start;
  String removed, inserted;
  final Sel before;
  Sel after;
  final int revBefore;
  int revAfter;
  final String kind;
  DateTime time = DateTime.now();
}

int _revisions = 0;

class EditorDocument extends ChangeNotifier {
  EditorDocument(String text, this.lang) : _lex = lexerFor(lang) {
    lines = [for (final l in text.replaceAll('\r\n', '\n').split('\n')) Line(l)];
    _relex(0, lines.length - 1);
  }

  final Lang lang;
  final LineLexer _lex;
  late List<Line> lines;
  Sel sel = const Sel.at(Pos(0, 0));

  /// Identifies the content; equal revisions mean equal text (undo returns
  /// to an earlier revision), so "unsaved" is `revision != savedRevision`.
  int revision = 0;
  List<Mark> marks = [];
  final _undo = <_Edit>[];
  final _redo = <_Edit>[];

  /// Bumped on every content change (for views that cache derived data).
  int version = 0;

  int get lineCount => lines.length;
  String lineText(int i) => lines[i].text;
  String get text => lines.map((l) => l.text).join('\n');
  bool get canUndo => _undo.isNotEmpty;
  bool get canRedo => _redo.isNotEmpty;

  int _maxLen = -1;
  int get maxLineLength {
    if (_maxLen < 0) _maxLen = lines.fold(0, (m, l) => math.max(m, l.text.length));
    return _maxLen;
  }

  Pos clamp(Pos p) {
    final l = p.line.clamp(0, lines.length - 1);
    return Pos(l, p.col.clamp(0, lines[l].text.length));
  }

  Pos get endPos => Pos(lines.length - 1, lines.last.text.length);

  String textIn(Pos a, Pos b) {
    if (b < a) (a, b) = (b, a);
    if (a.line == b.line) return lines[a.line].text.substring(a.col, b.col);
    final buf = StringBuffer(lines[a.line].text.substring(a.col));
    for (var i = a.line + 1; i < b.line; i++) {
      buf
        ..write('\n')
        ..write(lines[i].text);
    }
    buf
      ..write('\n')
      ..write(lines[b.line].text.substring(0, b.col));
    return buf.toString();
  }

  String get selectedText => textIn(sel.start, sel.end);

  void select(Sel s) {
    final c = Sel(clamp(s.anchor), clamp(s.head));
    if (c == sel) return;
    sel = c;
    notifyListeners();
  }

  /// Replaces `a..b` with `text` and returns the end of the inserted text.
  /// `kind` groups consecutive edits into one undo step ("type", "delete",
  /// "backspace"); other kinds always start a new step.
  Pos replace(Pos a, Pos b, String text, {String kind = 'edit', Sel? after}) {
    if (b < a) (a, b) = (b, a);
    a = clamp(a);
    b = clamp(b);
    text = text.replaceAll('\r\n', '\n').replaceAll('\r', '\n');
    final removed = textIn(a, b);
    if (removed.isEmpty && text.isEmpty) return a;
    final before = sel;
    final end = _apply(a, b, text);
    final newSel = after ?? Sel.at(end);
    final revBefore = revision;
    revision = ++_revisions;
    _redo.clear();
    final last = _undo.isEmpty ? null : _undo.last;
    final now = DateTime.now();
    final recent = last != null && now.difference(last.time) < const Duration(milliseconds: 1200) && last.kind == kind;
    final typingInside =
        recent &&
        kind == 'type' &&
        removed.isEmpty &&
        !text.contains('\n') &&
        !last.inserted.contains('\n') &&
        a.line == last.start.line &&
        a.col >= last.start.col &&
        a.col <= last.start.col + last.inserted.length;
    if (typingInside) {
      // Typing anywhere inside the current burst (e.g. between auto-closed
      // braces) stays one undo step.
      final at = a.col - last.start.col;
      last
        ..inserted = last.inserted.substring(0, at) + text + last.inserted.substring(at)
        ..after = newSel
        ..revAfter = revision
        ..time = now;
    } else if (recent && kind == 'backspace' && text.isEmpty && b == last.start && last.inserted.isEmpty) {
      last
        ..start = a
        ..removed = removed + last.removed
        ..after = newSel
        ..revAfter = revision
        ..time = now;
    } else if (recent && kind == 'delete' && text.isEmpty && a == last.start && last.inserted.isEmpty) {
      last
        ..removed += removed
        ..after = newSel
        ..revAfter = revision
        ..time = now;
    } else {
      _undo.add(_Edit(a, removed, text, before, newSel, revBefore, revision, kind));
      if (_undo.length > 2000) _undo.removeAt(0);
    }
    sel = newSel;
    _changed();
    return end;
  }

  /// Replaces the selection (typing, pasting).
  Pos insert(String text, {String kind = 'edit'}) => replace(sel.start, sel.end, text, kind: kind);

  static Pos _endOf(Pos start, String text) {
    final nl = text.lastIndexOf('\n');
    if (nl < 0) return Pos(start.line, start.col + text.length);
    return Pos(start.line + '\n'.allMatches(text).length, text.length - nl - 1);
  }

  Pos _apply(Pos a, Pos b, String text) {
    final head = lines[a.line].text.substring(0, a.col);
    final tail = lines[b.line].text.substring(b.col);
    final parts = text.split('\n');
    final removedLines = b.line - a.line;
    final first = lines[a.line];
    if (parts.length == 1) {
      first.text = head + parts[0] + tail;
      first.stateOut = -1;
      if (removedLines > 0) lines.removeRange(a.line + 1, b.line + 1);
    } else {
      first.text = head + parts[0];
      first.stateOut = -1;
      final added = [for (var i = 1; i < parts.length - 1; i++) Line(parts[i]), Line(parts.last + tail)];
      lines.replaceRange(a.line + 1, b.line + 1, added);
    }
    final end = Pos(a.line + parts.length - 1, (parts.length == 1 ? head.length : 0) + parts.last.length);
    _moveMarks(a, b, end);
    _relex(a.line, end.line);
    if (_maxLen >= 0) {
      var grew = false;
      for (var i = a.line; i <= end.line; i++) {
        if (lines[i].text.length > _maxLen) {
          _maxLen = lines[i].text.length;
          grew = true;
        }
      }
      if (!grew && removedLines > 0) _maxLen = -1; // a long line may be gone
    }
    return end;
  }

  /// Keeps diagnostics on their text: marks after the edit shift with it,
  /// marks the edit touches are dropped (they described the old text).
  void _moveMarks(Pos a, Pos b, Pos end) {
    if (marks.isEmpty) return;
    final dl = end.line - b.line;
    final out = <Mark>[];
    for (final m in marks) {
      if (m.end <= a) {
        out.add(m);
      } else if (m.start >= b) {
        Pos shift(Pos p) => p.line == b.line ? Pos(end.line, end.col + p.col - b.col) : Pos(p.line + dl, p.col);
        m
          ..start = shift(m.start)
          ..end = shift(m.end);
        out.add(m);
      }
    }
    marks = out;
  }

  void _relex(int from, int to) {
    var state = from == 0 ? 0 : lines[from - 1].stateOut;
    for (var i = from; i < lines.length; i++) {
      final l = lines[i];
      final old = l.stateOut;
      final out = <int>[];
      final s = _lex(l.text, state, out);
      if (i <= to || !listEquals(out, l.tokens)) {
        // Edited lines always re-layout; later lines only if their colours changed.
        l.tokens = out;
        l.layout = null;
      }
      l.stateOut = s;
      state = s;
      if (i >= to && s == old) break;
    }
  }

  void _changed() {
    version++;
    notifyListeners();
  }

  void undo() {
    if (_undo.isEmpty) return;
    final e = _undo.removeLast();
    _apply(e.start, _endOf(e.start, e.inserted), e.removed);
    _redo.add(e);
    revision = e.revBefore;
    sel = e.before;
    _changed();
  }

  void redo() {
    if (_redo.isEmpty) return;
    final e = _redo.removeLast();
    _apply(e.start, _endOf(e.start, e.removed), e.inserted);
    _undo.add(e..time = DateTime(0)); // never merge into a redone step
    revision = e.revAfter;
    sel = e.after;
    _changed();
  }

  /// Ends the current undo group (e.g. after moving the caret).
  void breakUndoGroup() {
    if (_undo.isNotEmpty) _undo.last.time = DateTime(0);
  }

  // ── Diagnostics ─────────────────────────────────────────────────────────

  /// Sets diagnostics from the core (1-based line/column).
  void setDiagnostics(List<Map<String, dynamic>> diags) {
    final out = <Mark>[];
    for (final d in diags) {
      final line = d['line'] as int?;
      if (line == null || line < 1 || line > lines.length) continue;
      final l = line - 1;
      final col = d['column'] as int?;
      Pos s, e;
      if (col != null) {
        s = clamp(Pos(l, col - 1));
        e = clamp(Pos(((d['endLine'] as int?) ?? line) - 1, ((d['endColumn'] as int?) ?? col + 1) - 1));
        if (e <= s) e = clamp(Pos(l, s.col + 1));
      } else {
        // TeX only knows the line: mark its content without indentation.
        final t = lines[l].text;
        final indent = t.length - t.trimLeft().length;
        s = Pos(l, indent);
        e = Pos(l, t.trimRight().length);
        if (e <= s) e = Pos(l, math.max(s.col, t.length));
      }
      out.add(Mark(s, e, d['severity'] as String? ?? 'error', d['message'] as String? ?? ''));
    }
    out.sort((a, b) => a.start.compareTo(b.start));
    marks = out;
    notifyListeners();
  }

  // ── Text helpers ────────────────────────────────────────────────────────

  static int _class(int c) {
    if (c == 32 || c == 9) return 0;
    if ((c >= 48 && c <= 57) || (c >= 65 && c <= 90) || (c >= 97 && c <= 122) || c == 95 || c >= 0xC0) return 1;
    return 2;
  }

  /// Start of the word left of `p` (Ctrl+Left, Ctrl+Backspace).
  Pos wordLeft(Pos p) {
    if (p.col == 0) return p.line == 0 ? p : Pos(p.line - 1, lines[p.line - 1].text.length);
    final t = lines[p.line].text;
    var i = p.col;
    while (i > 0 && _class(t.codeUnitAt(i - 1)) == 0) {
      i--;
    }
    if (i == 0) return Pos(p.line, 0);
    final k = _class(t.codeUnitAt(i - 1));
    while (i > 0 && _class(t.codeUnitAt(i - 1)) == k) {
      i--;
    }
    return Pos(p.line, i);
  }

  /// End of the word right of `p` (Ctrl+Right, Ctrl+Delete).
  Pos wordRight(Pos p) {
    final t = lines[p.line].text;
    if (p.col >= t.length) return p.line + 1 < lines.length ? Pos(p.line + 1, 0) : p;
    var i = p.col;
    while (i < t.length && _class(t.codeUnitAt(i)) == 0) {
      i++;
    }
    if (i == t.length) return Pos(p.line, i);
    final k = _class(t.codeUnitAt(i));
    while (i < t.length && _class(t.codeUnitAt(i)) == k) {
      i++;
    }
    return Pos(p.line, i);
  }

  /// The word around `p` (double-click).
  Sel wordAt(Pos p) {
    final t = lines[p.line].text;
    if (t.isEmpty) return Sel.at(p);
    var c = p.col.clamp(0, t.length - 1);
    if (p.col == t.length || (c > 0 && _class(t.codeUnitAt(c)) == 0 && _class(t.codeUnitAt(c - 1)) != 0)) c--;
    final k = _class(t.codeUnitAt(c));
    var s = c, e = c + 1;
    while (s > 0 && _class(t.codeUnitAt(s - 1)) == k) {
      s--;
    }
    while (e < t.length && _class(t.codeUnitAt(e)) == k) {
      e++;
    }
    return Sel(Pos(p.line, s), Pos(p.line, e));
  }

  /// Column one character left/right of `col`, skipping surrogate pairs.
  int stepLeft(String t, int col) => col >= 2 && (t.codeUnitAt(col - 1) & 0xFC00) == 0xDC00 ? col - 2 : col - 1;
  int stepRight(String t, int col) => col + 2 <= t.length && (t.codeUnitAt(col) & 0xFC00) == 0xD800 ? col + 2 : col + 1;

  static const _open = '([{', _close = ')]}';

  /// The bracket pair touching the caret, if any (for highlighting).
  (Pos, Pos)? matchingBracket(Pos p) {
    final t = lines[p.line].text;
    for (final c in [p.col - 1, p.col]) {
      if (c < 0 || c >= t.length) continue;
      final ch = t[c];
      final o = _open.indexOf(ch), cl = _close.indexOf(ch);
      if (o >= 0) {
        final m = _scan(Pos(p.line, c), _open[o], _close[o], 1);
        if (m != null) return (Pos(p.line, c), m);
      } else if (cl >= 0) {
        final m = _scan(Pos(p.line, c), _close[cl], _open[cl], -1);
        if (m != null) return (m, Pos(p.line, c));
      }
    }
    return null;
  }

  Pos? _scan(Pos from, String self, String other, int dir) {
    var depth = 0;
    var l = from.line, c = from.col;
    for (var steps = 0; steps < 20000; steps++) {
      final t = lines[l].text;
      if (c >= 0 && c < t.length) {
        final ch = t[c];
        if (ch == self) depth++;
        if (ch == other) {
          depth--;
          if (depth == 0) return Pos(l, c);
        }
      }
      c += dir;
      if (c < 0 || c >= t.length) {
        l += dir;
        if (l < 0 || l >= lines.length) return null;
        c = dir > 0 ? 0 : lines[l].text.length - 1;
        if (lines[l].text.isEmpty) c = dir > 0 ? 0 : -1;
      }
    }
    return null;
  }

  String get commentMark => lang == Lang.typst ? '// ' : '% ';
}
