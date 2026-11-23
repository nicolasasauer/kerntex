import 'package:flutter_test/flutter_test.dart';
import 'package:kerntex/src/editor/document.dart';
import 'package:kerntex/src/editor/highlight.dart';

List<int> _toks(EditorDocument d, int line) => d.lines[line].tokens;

void main() {
  test('language follows the file extension', () {
    expect(langFor('main.typ'), Lang.typst);
    expect(langFor('chapters/intro.TEX'), Lang.latex);
    expect(langFor('refs.bib'), Lang.bib);
    expect(langFor('notes.txt'), Lang.plain);
  });

  test('replace across lines and back with undo/redo', () {
    final d = EditorDocument('alpha\nbeta\ngamma', Lang.plain);
    final rev0 = d.revision;
    final end = d.replace(const Pos(0, 2), const Pos(2, 1), 'X\nY');
    expect(d.text, 'alX\nYamma');
    expect(end, const Pos(1, 1));
    expect(d.maxLineLength, 5);
    d.undo();
    expect(d.text, 'alpha\nbeta\ngamma');
    expect(d.revision, rev0, reason: 'undo returns to the saved revision');
    d.redo();
    expect(d.text, 'alX\nYamma');
    expect(d.textIn(const Pos(0, 1), const Pos(1, 2)), 'lX\nYa');
  });

  test('typing groups into one undo step, a newline starts a new one', () {
    final d = EditorDocument('', Lang.plain);
    for (final c in 'hello'.split('')) {
      d.insert(c, kind: 'type');
    }
    d.insert('\n', kind: 'newline');
    for (final c in 'you'.split('')) {
      d.insert(c, kind: 'type');
    }
    expect(d.text, 'hello\nyou');
    d.undo();
    expect(d.text, 'hello\n');
    d.undo();
    expect(d.text, 'hello');
    d.undo();
    expect(d.text, '');
    expect(d.canUndo, isFalse);
  });

  test('backspace runs group and restore together', () {
    final d = EditorDocument('abcdef', Lang.plain)..select(const Sel.at(Pos(0, 6)));
    for (var i = 0; i < 3; i++) {
      final p = d.sel.head;
      d.replace(Pos(0, p.col - 1), p, '', kind: 'backspace');
    }
    expect(d.text, 'abc');
    d.undo();
    expect(d.text, 'abcdef');
    expect(d.sel.head, const Pos(0, 6));
  });

  test('diagnostics follow the text they describe', () {
    final d = EditorDocument('#set page()\n  #foo(1)\nend', Lang.typst);
    d.setDiagnostics([
      {'severity': 'error', 'line': 2, 'column': 4, 'endLine': 2, 'endColumn': 7, 'message': 'x'},
      {'severity': 'warning', 'line': 3, 'message': 'w'},
      {'severity': 'info', 'line': null, 'message': 'none'},
    ]);
    expect(d.marks.map((m) => (m.start, m.end, m.severity)), [
      (const Pos(1, 3), const Pos(1, 6), 'error'),
      (const Pos(2, 0), const Pos(2, 3), 'warning'),
    ]);
    // Two lines inserted above: both marks move down.
    d.replace(const Pos(0, 0), const Pos(0, 0), 'a\nb\n');
    expect(d.marks.map((m) => m.start), [const Pos(3, 3), const Pos(4, 0)]);
    // Editing inside the error drops it; the warning stays.
    d.replace(const Pos(3, 4), const Pos(3, 5), 'X');
    expect(d.marks.map((m) => m.severity), ['warning']);
  });

  test('highlighting is incremental and carries state across lines', () {
    final d = EditorDocument('a \$x\ny\$ b\nc', Lang.latex);
    // Line 1 starts inside math.
    expect(_toks(d, 1).sublist(0, 3), [0, 2, Tok.math.index]);
    // Closing the math on line 0 changes how line 1 is read.
    d.replace(const Pos(0, 4), const Pos(0, 4), r'$');
    expect(_toks(d, 1), [1, 4, Tok.math.index], reason: 'line 1 now opens math instead');
    expect(d.lines[1].stateOut, 1);
  });

  test('typst block comments and raw blocks span lines', () {
    final d = EditorDocument('/* a\nb */ #set\n```\ncode\n```\n= Head', Lang.typst);
    expect(_toks(d, 1).sublist(0, 3), [0, 4, Tok.comment.index]);
    expect(_toks(d, 3), [0, 4, Tok.string.index]);
    expect(_toks(d, 5), [0, 6, Tok.heading.index]);
  });

  test('latex verbatim bodies are not highlighted', () {
    final d = EditorDocument('\\begin{verbatim}\n\\textbf{x} % no\n\\end{verbatim}\n\\textbf{y}', Lang.latex);
    expect(_toks(d, 1), [0, 15, Tok.string.index]);
    expect(_toks(d, 3).first, 0);
    expect(_toks(d, 3)[2], Tok.command.index);
  });

  test('editing a line without tokens still invalidates its layout', () {
    final d = EditorDocument('plain', Lang.plain);
    d.lines[0].layout = Object();
    d.replace(const Pos(0, 5), const Pos(0, 5), ' text');
    expect(d.lines[0].layout, isNull);
  });

  test('word motion and selection', () {
    final d = EditorDocument(r'  \section{Intro text}', Lang.latex);
    expect(d.wordRight(const Pos(0, 0)), const Pos(0, 3));
    expect(d.wordRight(const Pos(0, 3)), const Pos(0, 10));
    expect(d.wordLeft(const Pos(0, 16)), const Pos(0, 11));
    final w = d.wordAt(const Pos(0, 13));
    expect(d.textIn(w.start, w.end), 'Intro');
  });

  test('bracket matching across lines', () {
    final d = EditorDocument('f(a,\n  [b])', Lang.plain);
    expect(d.matchingBracket(const Pos(0, 2)), (const Pos(0, 1), const Pos(1, 5)));
    expect(d.matchingBracket(const Pos(1, 5)), (const Pos(1, 2), const Pos(1, 4)));
  });

  test('large documents: an edit re-lexes only what changed', () {
    final big = List.generate(100000, (i) => i.isEven ? r'\section{Title} with $x^2$ math % note' : 'plain text line $i').join('\n');
    final sw = Stopwatch()..start();
    final d = EditorDocument(big, Lang.latex);
    final open = sw.elapsedMilliseconds;
    sw.reset();
    for (var i = 0; i < 1000; i++) {
      d.insert('a', kind: 'type');
    }
    final perKey = sw.elapsedMicroseconds / 1000;
    // ignore: avoid_print
    print('100k lines: open ${open}ms, ${perKey.toStringAsFixed(1)}µs per keystroke');
    expect(perKey, lessThan(500));
    expect(d.lineCount, 100000);
  });
}
