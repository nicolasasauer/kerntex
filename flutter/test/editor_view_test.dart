import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:kerntex/src/editor/completion.dart';
import 'package:kerntex/src/editor/document.dart';
import 'package:kerntex/src/editor/editor_view.dart';
import 'package:kerntex/src/editor/highlight.dart';
import 'package:kerntex/src/theme.dart';
import 'package:material_ui/material_ui.dart';

Future<(EditorDocument, GlobalKey<EditorViewState>)> _pump(
  WidgetTester t,
  String text, {
  Lang lang = Lang.latex,
  CompletionSource? src,
}) async {
  final doc = EditorDocument(text, lang);
  final key = GlobalKey<EditorViewState>();
  final focus = FocusNode();
  await t.pumpWidget(
    MaterialApp(
      theme: buildTheme(Brightness.light),
      home: Scaffold(
        body: EditorView(key: key, doc: doc, fontSize: 14, focusNode: focus, source: src),
      ),
    ),
  );
  focus.requestFocus();
  await t.pump();
  return (doc, key);
}

/// Types like a platform text input does: it keeps its own copy of the
/// editing value (replaced whenever the editor sends a new one), applies the
/// keystroke to it and reports the result.
Map<String, dynamic>? _lastSeen;
TextEditingValue _platform = TextEditingValue.empty;

Future<void> _type(WidgetTester t, String s) async {
  for (final ch in s.split('')) {
    final st = t.testTextInput.editingState!;
    if (!identical(st, _lastSeen)) {
      _lastSeen = st;
      _platform = TextEditingValue(
        text: st['text'] as String,
        selection: TextSelection(baseOffset: st['selectionBase'] as int, extentOffset: st['selectionExtent'] as int),
      );
    }
    final sel = _platform.selection;
    final nt = _platform.text.replaceRange(sel.start, sel.end, ch);
    _platform = TextEditingValue(
      text: nt,
      selection: TextSelection.collapsed(offset: sel.start + ch.length),
    );
    t.testTextInput.updateEditingValue(_platform);
    await t.pump();
  }
}

Future<void> _key(WidgetTester t, LogicalKeyboardKey k, {bool ctrl = false, bool shift = false}) async {
  if (ctrl) await t.sendKeyDownEvent(LogicalKeyboardKey.controlLeft);
  if (shift) await t.sendKeyDownEvent(LogicalKeyboardKey.shiftLeft);
  await t.sendKeyEvent(k);
  if (shift) await t.sendKeyUpEvent(LogicalKeyboardKey.shiftLeft);
  if (ctrl) await t.sendKeyUpEvent(LogicalKeyboardKey.controlLeft);
  await t.pump();
}

void main() {
  testWidgets('typing, auto-closing braces and undo', (t) async {
    final (doc, _) = await _pump(t, '');
    await _type(t, r'\textbf{');
    expect(doc.text, r'\textbf{}', reason: 'brace closed automatically');
    await _type(t, 'bold}');
    expect(doc.text, r'\textbf{bold}', reason: 'typing } steps over the closing brace');
    expect(doc.sel.head, const Pos(0, 13));
    await _key(t, LogicalKeyboardKey.keyZ, ctrl: true);
    expect(doc.text, isEmpty, reason: 'one typing burst is one undo step');
    await _key(t, LogicalKeyboardKey.keyZ, ctrl: true, shift: true);
    expect(doc.text, r'\textbf{bold}');
  });

  testWidgets('Enter keeps indentation and closes \\begin environments', (t) async {
    final (doc, _) = await _pump(t, '');
    await _type(t, r'  \begin{itemize}');
    await _key(t, LogicalKeyboardKey.enter);
    expect(doc.text, '  \\begin{itemize}\n    \n  \\end{itemize}');
    expect(doc.sel.head, const Pos(1, 4));
    await _type(t, r'\item a');
    await _key(t, LogicalKeyboardKey.enter);
    expect(doc.lines[2].text, '    ', reason: 'indentation carried over');
  });

  testWidgets('keyboard selection, deletion and line commands', (t) async {
    final (doc, _) = await _pump(t, 'one two three\nfour');
    await _key(t, LogicalKeyboardKey.end, shift: true);
    expect(doc.selectedText, 'one two three');
    await _key(t, LogicalKeyboardKey.home);
    await _key(t, LogicalKeyboardKey.arrowRight, ctrl: true, shift: true);
    expect(doc.selectedText, 'one');
    await _key(t, LogicalKeyboardKey.delete);
    expect(doc.lines[0].text, ' two three');
    await _key(t, LogicalKeyboardKey.backspace, ctrl: true); // nothing left of the caret on line 0
    await _key(t, LogicalKeyboardKey.arrowDown);
    await _key(t, LogicalKeyboardKey.slash, ctrl: true);
    expect(doc.lines[1].text, '% four');
    await _key(t, LogicalKeyboardKey.slash, ctrl: true);
    expect(doc.lines[1].text, 'four');
    await _key(t, LogicalKeyboardKey.keyK, ctrl: true, shift: true);
    expect(doc.text, ' two three');
  });

  testWidgets('tab indents a selection, shift+tab outdents it', (t) async {
    final (doc, _) = await _pump(t, 'a\nb\nc');
    doc.select(const Sel(Pos(0, 0), Pos(2, 1)));
    await _key(t, LogicalKeyboardKey.tab);
    expect(doc.text, '  a\n  b\n  c');
    await _key(t, LogicalKeyboardKey.tab, shift: true);
    expect(doc.text, 'a\nb\nc');
  });

  testWidgets('completion: commands, environments and references', (t) async {
    final src = CompletionSource()
      ..labels = [('fig:cat', 'main.tex:3'), ('sec:intro', 'main.tex:1')]
      ..citations = [('knuth1984', 'The TeXbook')];
    final (doc, key) = await _pump(t, '', src: src);
    await _type(t, r'\subsec');
    expect(key.currentState!.completion.open, isTrue);
    expect(key.currentState!.completion.selected!.label, 'subsection');
    await _key(t, LogicalKeyboardKey.enter);
    expect(doc.text, r'\subsection{}');
    expect(doc.sel.head, const Pos(0, 12), reason: 'caret inside the braces');

    doc.select(Sel(doc.endPos, doc.endPos));
    await _key(t, LogicalKeyboardKey.end);
    await _key(t, LogicalKeyboardKey.enter);
    await _type(t, r'\ref{fig');
    expect(key.currentState!.completion.selected!.label, 'fig:cat');
    await _key(t, LogicalKeyboardKey.tab);
    expect(doc.lines[1].text, r'\ref{fig:cat}');

    await _key(t, LogicalKeyboardKey.end);
    await _key(t, LogicalKeyboardKey.enter);
    await _type(t, r'\begin{enum');
    await _key(t, LogicalKeyboardKey.enter);
    expect(doc.lines.skip(2).map((l) => l.text).join('\n'), '\\begin{enumerate}\n  \\item \n\\end{enumerate}');
    expect(doc.sel.head, const Pos(3, 8));
  });

  testWidgets('typst completion offers functions and @ references', (t) async {
    final src = CompletionSource()..labels = [('intro', 'main.typ:2')];
    final (doc, key) = await _pump(t, '', lang: Lang.typst, src: src);
    await _type(t, '#ima');
    expect(key.currentState!.completion.selected!.label, 'image');
    await _key(t, LogicalKeyboardKey.enter);
    expect(doc.text, '#image("")');
    expect(doc.sel.head, const Pos(0, 8));
    await _key(t, LogicalKeyboardKey.end);
    await _type(t, ' see @in');
    expect(key.currentState!.completion.selected!.label, 'intro');
  });

  testWidgets('find and replace', (t) async {
    final (doc, key) = await _pump(t, 'cat dog cat\nCat');
    await _key(t, LogicalKeyboardKey.keyH, ctrl: true);
    final s = key.currentState!;
    expect(s.find.open, isTrue);
    s.find.query.text = 'cat';
    s.find.recompute(doc);
    expect(s.find.matches.length, 3, reason: 'case-insensitive by default');
    s.find.caseSensitive = true;
    s.find.recompute(doc);
    expect(s.find.matches.length, 2);
    s.find.replacement.text = 'bird';
    s.replaceAll();
    expect(doc.text, 'bird dog bird\nCat');
    doc.undo();
    expect(doc.text, 'cat dog cat\nCat', reason: 'replace all is one undo step');
  });

  testWidgets('only visible lines are laid out', (t) async {
    final big = List.generate(
      50000,
      (i) =>
          r'\section{Line} $x$ % '
          '$i',
    ).join('\n');
    final (doc, key) = await _pump(t, big);
    final laidOut = doc.lines.where((l) => l.layout != null).length;
    expect(laidOut, lessThan(120), reason: 'a screenful, not 50 000 lines');
    key.currentState!.reveal(40000);
    await t.pump();
    await t.pump();
    expect(doc.lines[39999].layout, isNotNull);
    expect(doc.lines.where((l) => l.layout != null).length, lessThan(240));
    // Scrolling through the whole file keeps the layout cache bounded.
    for (var line = 1; line < 50000; line += 40) {
      key.currentState!.reveal(line);
      await t.pump();
    }
    expect(doc.lines.where((l) => l.layout != null).length, lessThanOrEqualTo(1000));
  });
}
