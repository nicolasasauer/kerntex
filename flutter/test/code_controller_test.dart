import 'package:flutter_test/flutter_test.dart';
import 'package:kerntex/src/editor/code_controller.dart';

void main() {
  test('language follows the file extension', () {
    expect(langFor('main.typ'), Lang.typst);
    expect(langFor('chapters/intro.TEX'), Lang.latex);
    expect(langFor('refs.bib'), Lang.bib);
    expect(langFor('notes.txt'), Lang.plain);
  });

  test('line and offset arithmetic', () {
    final c = CodeController(text: 'ab\ncde\n\nf', lang: Lang.plain);
    expect(c.lineCount, 4);
    expect(c.maxLineLength, 3);
    expect(c.lineOf(0), 0);
    expect(c.lineOf(4), 1);
    expect(c.lineOf(8), 3);
    expect(c.offsetOf(2, 2), 4);
    expect(c.offsetOf(99), 8, reason: 'clamped to the last line');
    expect(c.offsetOf(1, 99), 2, reason: 'clamped to the end of the line');
  });

  test('diagnostics become marks and vanish after an edit', () {
    final c = CodeController(text: '#set page()\n  #foo(1)\n', lang: Lang.typst);
    c.setDiagnostics([
      {'severity': 'error', 'line': 2, 'column': 4, 'endLine': 2, 'endColumn': 7},
      {'severity': 'warning', 'line': 2},
      {'severity': 'info', 'line': null},
    ]);
    expect(c.marks.map((m) => (m.start, m.end, m.severity)), [(14, 21, 'warning'), (15, 18, 'error')]);
    c.text = '${c.text}x';
    expect(c.marks, isEmpty);
  });
}
