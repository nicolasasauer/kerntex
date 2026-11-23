// KernTeX's code editor.
//
// * Virtualised: only the lines on screen are laid out and painted. Each line
//   keeps its own cached `ui.Paragraph`, so scrolling and typing re-lay-out a
//   single line, never the document.
// * Text input goes through Flutter's own `TextInputClient` (the interface its
//   text fields use), so keyboard layouts, dead keys and IMEs behave as in
//   any Flutter text field. The platform only ever sees the current line.
// * Everything else — selection, motion, undo, brackets, find/replace,
//   completion — is implemented here for LaTeX and Typst.

import 'dart:async';
import 'dart:math' as math;
import 'dart:ui' as ui;

import 'package:flutter/gestures.dart';
import 'package:flutter/services.dart';
import 'package:material_ui/material_ui.dart';

import '../theme.dart';
import 'completion.dart';
import 'document.dart';
import 'find_bar.dart';
import 'highlight.dart';

const _padTop = 10.0;
const _padLeft = 12.0;

class EditorView extends StatefulWidget {
  const EditorView({super.key, required this.doc, required this.fontSize, required this.focusNode, this.source});

  final EditorDocument doc;
  final double fontSize;
  final FocusNode focusNode;
  final CompletionSource? source;

  @override
  State<EditorView> createState() => EditorViewState();
}

class EditorViewState extends State<EditorView> with TextInputClient {
  final _v = ScrollController();
  final _h = ScrollController();
  final _contentKey = GlobalKey();
  final _caretOn = ValueNotifier(true);
  final find = FindState();
  final completion = CompletionState();
  Timer? _blink, _autoScroll;
  TextInputConnection? _conn;
  TextEditingValue _sent = TextEditingValue.empty;
  int _sentLine = 0;
  TextRange _composing = TextRange.empty;
  bool _applyingInput = false;
  double? _goalX;
  double _viewportH = 600, _viewportW = 800;

  // Mouse
  int _clicks = 0;
  DateTime _lastDown = DateTime(0);
  Offset _lastDownPos = Offset.zero;
  Sel? _dragOrigin;
  Offset? _dragViewport;

  // Metrics
  double _lh = 22, _charW = 8;
  int _gen = 0;
  double _metricsFor = 0;
  KT? _themeFor;
  late ui.TextStyle _base;
  late List<ui.TextStyle> _tokStyles;

  EditorDocument get doc => widget.doc;

  // ── Lifecycle ───────────────────────────────────────────────────────────

  @override
  void initState() {
    super.initState();
    doc.addListener(_docChanged);
    widget.focusNode.addListener(_focusChanged);
    find.addListener(_findChanged);
    if (widget.focusNode.hasFocus) _focusChanged();
  }

  @override
  void didUpdateWidget(EditorView oldWidget) {
    super.didUpdateWidget(oldWidget);
    final old = oldWidget;
    if (old.doc != doc) {
      old.doc.removeListener(_docChanged);
      doc.addListener(_docChanged);
      completion.close();
      find.recompute(doc);
      _syncInput();
    }
    if (old.focusNode != widget.focusNode) {
      old.focusNode.removeListener(_focusChanged);
      widget.focusNode.addListener(_focusChanged);
    }
  }

  @override
  void dispose() {
    doc.removeListener(_docChanged);
    widget.focusNode.removeListener(_focusChanged);
    _conn?.close();
    _blink?.cancel();
    _autoScroll?.cancel();
    _v.dispose();
    _h.dispose();
    find.dispose();
    completion.dispose();
    _caretOn.dispose();
    super.dispose();
  }

  void _docChanged() {
    if (!mounted) return;
    _restartBlink();
    if (!_applyingInput) _syncInput();
    find.docChanged(doc);
    setState(() {});
  }

  void _findChanged() => setState(() {});

  void _focusChanged() {
    if (widget.focusNode.hasFocus) {
      _openInput();
      _restartBlink();
    } else {
      _conn?.close();
      _conn = null;
      _blink?.cancel();
      _caretOn.value = false;
      completion.close();
    }
    setState(() {});
  }

  void _restartBlink() {
    _blink?.cancel();
    _caretOn.value = true;
    if (!widget.focusNode.hasFocus) return;
    _blink = Timer.periodic(const Duration(milliseconds: 530), (_) => _caretOn.value = !_caretOn.value);
  }

  // ── Metrics and line layout ─────────────────────────────────────────────

  void _metrics(KT k) {
    if (_metricsFor == widget.fontSize && identical(_themeFor, k)) return;
    _metricsFor = widget.fontSize;
    _themeFor = k;
    _gen++;
    _lh = (widget.fontSize * 1.6).roundToDouble();
    ui.TextStyle st(Color c, {FontWeight? w, FontStyle? s}) => ui.TextStyle(
      color: c,
      fontFamily: monoFamily,
      fontFamilyFallback: monoFallback,
      fontSize: widget.fontSize,
      fontWeight: w,
      fontStyle: s,
    );
    _base = st(k.text);
    _tokStyles = [
      st(k.synCommand),
      st(k.synKeyword, w: FontWeight.w600),
      st(k.synAtom),
      st(k.synNumber),
      st(k.synMath),
      st(k.synComment, s: FontStyle.italic),
      st(k.synBracket),
      st(k.synString),
      st(k.accent, w: FontWeight.w700),
      st(k.text, w: FontWeight.w700),
      st(k.text, s: FontStyle.italic),
    ];
    final p = _build('MMMMMMMMMMMMMMMMMMMM', const []);
    _charW = p.maxIntrinsicWidth / 20;
  }

  ui.Paragraph _build(String text, List<int> tokens) {
    final b = ui.ParagraphBuilder(
      ui.ParagraphStyle(
        maxLines: 1,
        strutStyle: ui.StrutStyle(
          fontFamily: monoFamily,
          fontFamilyFallback: monoFallback,
          fontSize: widget.fontSize,
          height: 1.6,
          forceStrutHeight: true,
        ),
      ),
    )..pushStyle(_base);
    var at = 0;
    for (var i = 0; i + 2 < tokens.length; i += 3) {
      final s = tokens[i], e = tokens[i + 1];
      if (s < at) continue;
      if (s > at) b.addText(text.substring(at, s));
      b
        ..pushStyle(_tokStyles[tokens[i + 2]])
        ..addText(text.substring(s, e))
        ..pop();
      at = e;
    }
    if (at < text.length) b.addText(text.substring(at));
    return b.build()..layout(const ui.ParagraphConstraints(width: 1e7));
  }

  /// Lines that currently hold a layout, oldest first. Capped, so scrolling
  /// through a huge file keeps memory flat.
  final _laidOut = <Line>[];
  static const _maxLaidOut = 800;

  ui.Paragraph _para(int line) {
    final l = doc.lines[line];
    if (l.layout != null && l.layoutGen == _gen) return l.layout as ui.Paragraph;
    final p = _build(l.text, l.tokens);
    l
      ..layout = p
      ..layoutGen = _gen;
    _laidOut.add(l);
    if (_laidOut.length > _maxLaidOut * 1.25) {
      // Drop the oldest quarter (screens of lines scrolled past long ago).
      final drop = _laidOut.length - _maxLaidOut;
      for (final old in _laidOut.take(drop)) {
        // Not disposed explicitly: a picture painted this frame may still use it.
        if (!identical(old, l)) old.layout = null;
      }
      _laidOut.removeRange(0, drop);
    }
    return p;
  }

  /// X offset of column `col` in `line` (exact, also for wide characters).
  double xOf(int line, int col) {
    if (col <= 0) return 0;
    final t = doc.lines[line].text;
    col = math.min(col, t.length);
    final p = _para(line);
    var boxes = p.getBoxesForRange(col - 1, col);
    if (boxes.isEmpty && col >= 2) boxes = p.getBoxesForRange(col - 2, col);
    return boxes.isEmpty ? col * _charW : boxes.last.right;
  }

  int _colAt(int line, double x) {
    if (x <= 0) return 0;
    final p = _para(line);
    final pos = p.getPositionForOffset(Offset(x, _lh / 2));
    return pos.offset.clamp(0, doc.lines[line].text.length);
  }

  Pos _posAt(Offset local) {
    final line = ((local.dy - _padTop) / _lh).floor().clamp(0, doc.lineCount - 1);
    return Pos(line, _colAt(line, local.dx - _padLeft));
  }

  Rect caretRect(Pos p) => Rect.fromLTWH(_padLeft + xOf(p.line, p.col), _padTop + p.line * _lh, 2, _lh);

  // ── Scrolling ───────────────────────────────────────────────────────────

  void _ensureVisible(Pos p, {bool center = false}) {
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!_v.hasClients || !_h.hasClients) return;
      final r = caretRect(p);
      final vy = _v.offset;
      double? ty;
      if (center) {
        ty = r.top - _viewportH / 3;
      } else if (r.top < vy + _lh) {
        ty = r.top - _lh;
      } else if (r.bottom > vy + _viewportH - _lh) {
        ty = r.bottom - _viewportH + _lh;
      }
      if (ty != null) _v.jumpTo(ty.clamp(0, _v.position.maxScrollExtent));
      final hx = _h.offset;
      double? tx;
      if (r.left < hx + 40) tx = r.left - 40;
      if (r.right > hx + _viewportW - 40) tx = r.right - _viewportW + 40;
      if (tx != null) _h.jumpTo(tx.clamp(0, _h.position.maxScrollExtent));
      _updateImeRect();
    });
  }

  /// Puts the caret at 1-based `line`/`column` and scrolls it into view.
  void reveal(int line, [int? column]) {
    final p = doc.clamp(Pos(line - 1, (column ?? 1) - 1));
    doc.select(Sel.at(p));
    widget.focusNode.requestFocus();
    _ensureVisible(p, center: true);
  }

  // ── Text input (IME, dead keys, soft keyboards) ─────────────────────────

  void _openInput() {
    if (_conn == null || !_conn!.attached) {
      _conn = TextInput.attach(
        this,
        const TextInputConfiguration(
          inputType: TextInputType.multiline,
          inputAction: TextInputAction.newline,
          autocorrect: false,
          enableSuggestions: false,
          enableIMEPersonalizedLearning: false,
          smartDashesType: SmartDashesType.disabled,
          smartQuotesType: SmartQuotesType.disabled,
        ),
      );
      _conn!.show();
    }
    _syncInput();
  }

  void _syncInput() {
    final c = _conn;
    if (c == null || !c.attached) return;
    final s = doc.sel;
    _sentLine = s.head.line;
    final t = doc.lines[_sentLine].text;
    final sel = s.start.line == s.end.line
        ? TextSelection(baseOffset: s.anchor.col, extentOffset: s.head.col)
        : TextSelection.collapsed(offset: s.head.col);
    _composing = TextRange.empty;
    _sent = TextEditingValue(text: t, selection: sel);
    c.setEditingState(_sent);
    _updateImeRect();
  }

  void _updateImeRect() {
    final c = _conn;
    final box = _contentKey.currentContext?.findRenderObject() as RenderBox?;
    if (c == null || !c.attached || box == null || !box.attached) return;
    c.setEditableSizeAndTransform(box.size, box.getTransformTo(null));
    c.setCaretRect(caretRect(doc.sel.head));
  }

  @override
  TextEditingValue? get currentTextEditingValue => _sent;

  @override
  AutofillScope? get currentAutofillScope => null;

  @override
  void updateEditingValue(TextEditingValue v) {
    final old = _sent;
    _sent = v;
    final line = _sentLine;
    if (line >= doc.lineCount) return _syncInput();
    _composing = v.composing;
    if (v.text == old.text) {
      // Selection moved by the platform (e.g. a soft keyboard's cursor keys).
      if (v.selection.isValid && v.selection != old.selection && doc.sel.start.line == doc.sel.end.line) {
        doc.select(Sel(Pos(line, v.selection.baseOffset), Pos(line, v.selection.extentOffset)));
      }
      setState(() {});
      return;
    }
    // Diff against what the platform had: the changed span of this line.
    final a = old.text, b = v.text;
    var pre = 0;
    final maxPre = math.min(a.length, b.length);
    while (pre < maxPre && a.codeUnitAt(pre) == b.codeUnitAt(pre)) {
      pre++;
    }
    var suf = 0;
    while (suf < maxPre - pre && a.codeUnitAt(a.length - 1 - suf) == b.codeUnitAt(b.length - 1 - suf)) {
      suf++;
    }
    var from = Pos(line, pre), to = Pos(line, a.length - suf);
    final inserted = b.substring(pre, b.length - suf);
    final s = doc.sel;
    // A multi-line selection was never sent to the platform: replace it.
    if (!s.isEmpty && s.start.line != s.end.line && from == to) {
      from = s.start;
      to = s.end;
    }
    _applyingInput = true;
    try {
      if (inserted == '\n' && from == to && !v.composing.isValid) {
        _applyingInput = false;
        _newline();
        return;
      }
      if (inserted.length == 1 && from == to && !v.composing.isValid && _typeChar(inserted)) {
        _applyingInput = false;
        _syncInput();
        return;
      }
      final end = doc.replace(from, to, inserted, kind: v.composing.isValid ? 'compose' : (inserted.length == 1 ? 'type' : 'edit'));
      final caret = inserted.contains('\n')
          ? end
          : Pos(end.line, v.selection.isValid ? v.selection.extentOffset.clamp(0, doc.lines[end.line].text.length) : end.col);
      doc.select(Sel.at(caret));
    } finally {
      _applyingInput = false;
    }
    if (doc.sel.head.line != _sentLine || doc.lines[_sentLine].text != v.text) _syncInput();
    _goalX = null;
    _ensureVisible(doc.sel.head);
    if (inserted.length == 1 && !v.composing.isValid) _completeAfterTyping();
  }

  @override
  void performAction(TextInputAction action) {
    if (action == TextInputAction.newline) _newline();
  }

  @override
  void updateFloatingCursor(RawFloatingCursorPoint point) {}
  @override
  void showAutocorrectionPromptRect(int start, int end) {}
  @override
  void connectionClosed() {
    _conn = null;
  }

  @override
  void performPrivateCommand(String action, Map<String, dynamic> data) {}
  @override
  void insertTextPlaceholder(Size size) {}
  @override
  void removeTextPlaceholder() {}
  @override
  void showToolbar() {}
  @override
  void didChangeInputControl(TextInputControl? oldControl, TextInputControl? newControl) {}
  @override
  void performSelector(String selectorName) {}
  @override
  void insertContent(KeyboardInsertedContent content) {}

  // ── Editing commands ────────────────────────────────────────────────────

  static const _pairs = {'(': ')', '[': ']', '{': '}'};

  /// Typed characters with special handling: auto-closing brackets, typing
  /// over a closing bracket, wrapping a selection. Returns true if handled.
  bool _typeChar(String ch) {
    final s = doc.sel;
    final t = doc.lines[s.head.line].text;
    final next = s.head.col < t.length ? t[s.head.col] : '';
    final close = _pairs[ch] ?? (doc.lang == Lang.typst && ch == r'$' ? r'$' : null);
    if (close != null && !s.isEmpty) {
      final inner = doc.selectedText;
      final start = s.start;
      doc.replace(s.start, s.end, '$ch$inner$close', kind: 'edit');
      final innerEnd = _endAfter(start, inner, 1);
      doc.select(Sel(Pos(start.line, start.col + 1), innerEnd));
      return true;
    }
    if (_pairs.containsValue(ch) && next == ch && s.isEmpty) {
      doc.select(Sel.at(Pos(s.head.line, s.head.col + 1)));
      return true;
    }
    if (close != null && s.isEmpty && (next.isEmpty || ' \t)]}.,;'.contains(next))) {
      // Don't pair an escaped brace: \{ in LaTeX.
      if (doc.lang == Lang.latex && s.head.col > 0 && t[s.head.col - 1] == r'\') return false;
      doc.replace(s.head, s.head, '$ch$close', kind: 'type', after: Sel.at(Pos(s.head.line, s.head.col + 1)));
      return true;
    }
    return false;
  }

  /// End of `text` inserted at `start` after `shift` extra characters.
  static Pos _endAfter(Pos start, String text, int shift) {
    final nl = text.lastIndexOf('\n');
    if (nl < 0) return Pos(start.line, start.col + shift + text.length);
    return Pos(start.line + '\n'.allMatches(text).length, text.length - nl - 1);
  }

  void _newline() {
    final s = doc.sel;
    final t = doc.lines[s.start.line].text;
    final before = t.substring(0, s.start.col);
    final indent = RegExp(r'^[ \t]*').stringMatch(before) ?? '';
    final afterText = doc.lines[s.end.line].text.substring(s.end.col);
    final prevChar = before.isEmpty ? '' : before[before.length - 1];
    final nextChar = afterText.isEmpty ? '' : afterText[0];
    // Between a pair of brackets: open an indented block.
    if (s.isEmpty && _pairs[prevChar] == nextChar && nextChar.isNotEmpty) {
      doc.replace(s.start, s.end, '\n$indent  \n$indent', kind: 'newline');
      doc.select(Sel.at(Pos(s.start.line + 1, indent.length + 2)));
      _ensureVisible(doc.sel.head);
      return;
    }
    // LaTeX: Enter after \begin{env} adds the matching \end{env}.
    final m = RegExp(r'\\begin\{([^}]+)\}(\[[^\]]*\])?(\{[^}]*\})*\s*$').firstMatch(before);
    if (doc.lang == Lang.latex && m != null && afterText.trim().isEmpty && !_hasEnd(s.start.line, m.group(1)!, indent)) {
      final env = m.group(1)!;
      doc.replace(s.start, s.end, '\n$indent  \n$indent\\end{$env}', kind: 'newline');
      doc.select(Sel.at(Pos(s.start.line + 1, indent.length + 2)));
      _ensureVisible(doc.sel.head);
      return;
    }
    final extra = (prevChar == '{' || prevChar == '[' || prevChar == '(') ? '  ' : '';
    doc.replace(s.start, s.end, '\n$indent$extra', kind: 'newline');
    _ensureVisible(doc.sel.head);
  }

  bool _hasEnd(int line, String env, String indent) {
    for (var i = line + 1; i < math.min(doc.lineCount, line + 400); i++) {
      final t = doc.lines[i].text;
      if (t.contains('\\end{$env}')) return t.startsWith('$indent\\end') || t.trimLeft().startsWith('\\end');
      if (t.contains('\\begin{$env}')) return false;
    }
    return false;
  }

  void _backspace({bool word = false}) {
    final s = doc.sel;
    if (!s.isEmpty) {
      doc.replace(s.start, s.end, '', kind: 'edit');
      return;
    }
    final p = s.head;
    if (p.line == 0 && p.col == 0) return;
    final t = doc.lines[p.line].text;
    Pos from;
    var to = p;
    if (word) {
      from = doc.wordLeft(p);
    } else if (p.col == 0) {
      from = Pos(p.line - 1, doc.lines[p.line - 1].text.length);
    } else if (t.substring(0, p.col).trim().isEmpty && p.col >= 2 && p.col % 2 == 0 && t.substring(p.col - 2, p.col) == '  ') {
      from = Pos(p.line, p.col - 2); // soft tab
    } else {
      from = Pos(p.line, doc.stepLeft(t, p.col));
      // Delete an empty auto-closed pair at once.
      final ch = t.substring(from.col, p.col);
      if (_pairs[ch] != null && p.col < t.length && t[p.col] == _pairs[ch]) to = Pos(p.line, p.col + 1);
    }
    doc.replace(from, to, '', kind: 'backspace');
  }

  void _delete({bool word = false}) {
    final s = doc.sel;
    if (!s.isEmpty) {
      doc.replace(s.start, s.end, '', kind: 'edit');
      return;
    }
    final p = s.head;
    final t = doc.lines[p.line].text;
    final Pos to;
    if (word) {
      to = doc.wordRight(p);
    } else if (p.col >= t.length) {
      if (p.line + 1 >= doc.lineCount) return;
      to = Pos(p.line + 1, 0);
    } else {
      to = Pos(p.line, doc.stepRight(t, p.col));
    }
    doc.replace(p, to, '', kind: 'delete');
  }

  (int, int) _selLines() {
    final s = doc.sel;
    final last = (s.end.col == 0 && s.end.line > s.start.line) ? s.end.line - 1 : s.end.line;
    return (s.start.line, last);
  }

  /// Replaces whole lines `first..last` with `f(line)` as one undo step.
  void _mapLines(String Function(String) f, {required Sel Function(int first, List<String> out) selAfter}) {
    final (first, last) = _selLines();
    final out = [for (var i = first; i <= last; i++) f(doc.lines[i].text)];
    final a = Pos(first, 0), b = Pos(last, doc.lines[last].text.length);
    doc.replace(a, b, out.join('\n'), kind: 'edit', after: selAfter(first, out));
  }

  void _indent(bool outdent) {
    final s = doc.sel;
    if (s.isEmpty && !outdent) {
      final col = s.head.col;
      doc.insert(col.isOdd ? ' ' : '  ', kind: 'type');
      return;
    }
    final anchor = s.anchor, head = s.head;
    final removed = <int, int>{};
    _mapLines(
      (l) {
        if (!outdent) return l.isEmpty ? l : '  $l';
        final n = l.startsWith('  ') ? 2 : (l.startsWith(' ') || l.startsWith('\t') ? 1 : 0);
        removed[removed.length] = n;
        return l.substring(n);
      },
      selAfter: (first, out) {
        Pos move(Pos p) {
          final i = p.line - first;
          if (i < 0 || i >= out.length) return p;
          final d = outdent ? -(removed[i] ?? 0) : (out[i].isEmpty ? 0 : 2);
          return Pos(p.line, math.max(0, p.col + d));
        }

        return Sel(move(anchor), move(head));
      },
    );
  }

  void toggleComment() {
    final mark = doc.commentMark;
    final bare = mark.trim();
    final (first, last) = _selLines();
    final ls = [for (var i = first; i <= last; i++) doc.lines[i].text];
    final all = ls.where((l) => l.trim().isNotEmpty).every((l) => l.trimLeft().startsWith(bare));
    final minIndent = ls.where((l) => l.trim().isNotEmpty).fold<int>(1 << 30, (m, l) => math.min(m, l.length - l.trimLeft().length));
    _mapLines((l) {
      if (l.trim().isEmpty) return l;
      if (all) {
        final i = l.indexOf(bare);
        final cut = l.startsWith(mark, i) ? mark.length : bare.length;
        return l.substring(0, i) + l.substring(i + cut);
      }
      return l.substring(0, minIndent) + mark + l.substring(minIndent);
    }, selAfter: (first, out) => Sel(Pos(first, 0), Pos(first + out.length - 1, out.last.length)));
  }

  void _moveLines(int dir) {
    final (first, last) = _selLines();
    if ((dir < 0 && first == 0) || (dir > 0 && last >= doc.lineCount - 1)) return;
    final s = doc.sel;
    final block = [for (var i = first; i <= last; i++) doc.lines[i].text];
    if (dir < 0) {
      final above = doc.lines[first - 1].text;
      doc.replace(
        Pos(first - 1, 0),
        Pos(last, doc.lines[last].text.length),
        [...block, above].join('\n'),
        kind: 'edit',
        after: Sel(Pos(s.anchor.line - 1, s.anchor.col), Pos(s.head.line - 1, s.head.col)),
      );
    } else {
      final below = doc.lines[last + 1].text;
      doc.replace(
        Pos(first, 0),
        Pos(last + 1, below.length),
        [below, ...block].join('\n'),
        kind: 'edit',
        after: Sel(Pos(s.anchor.line + 1, s.anchor.col), Pos(s.head.line + 1, s.head.col)),
      );
    }
    _ensureVisible(doc.sel.head);
  }

  void _deleteLines() {
    final (first, last) = _selLines();
    final a = first > 0 && last == doc.lineCount - 1 ? Pos(first - 1, doc.lines[first - 1].text.length) : Pos(first, 0);
    final b = last + 1 < doc.lineCount ? Pos(last + 1, 0) : Pos(last, doc.lines[last].text.length);
    doc.replace(a, b, '', kind: 'edit');
  }

  Future<void> copy({bool cut = false}) async {
    final s = doc.sel;
    if (s.isEmpty) {
      // No selection: the whole line, like most code editors.
      final l = s.head.line;
      await Clipboard.setData(ClipboardData(text: '${doc.lines[l].text}\n'));
      if (cut) _deleteLines();
      return;
    }
    await Clipboard.setData(ClipboardData(text: doc.selectedText));
    if (cut) doc.insert('', kind: 'edit');
  }

  Future<void> paste() async {
    final data = await Clipboard.getData(Clipboard.kTextPlain);
    final t = data?.text;
    if (t == null || t.isEmpty) return;
    doc.insert(t, kind: 'paste');
    doc.breakUndoGroup();
    _ensureVisible(doc.sel.head);
  }

  void selectAll() => doc.select(Sel(const Pos(0, 0), doc.endPos));

  // ── Caret motion ────────────────────────────────────────────────────────

  void _move(Pos to, bool extend, {bool keepGoal = false}) {
    if (!keepGoal) _goalX = null;
    doc.breakUndoGroup();
    doc.select(extend ? Sel(doc.sel.anchor, to) : Sel.at(to));
    completion.close();
    _ensureVisible(to);
  }

  void _vertical(int lines, bool extend) {
    final s = doc.sel;
    if (!extend && !s.isEmpty && lines.abs() == 1) {
      final p = lines < 0 ? s.start : s.end;
      doc.select(Sel.at(p));
    }
    final h = doc.sel.head;
    _goalX ??= xOf(h.line, h.col);
    final target = h.line + lines;
    if (target < 0) return _move(const Pos(0, 0), extend, keepGoal: true);
    if (target >= doc.lineCount) return _move(doc.endPos, extend, keepGoal: true);
    _move(Pos(target, _colAt(target, _goalX!)), extend, keepGoal: true);
  }

  void _home(bool extend, bool docStart) {
    if (docStart) return _move(const Pos(0, 0), extend);
    final h = doc.sel.head;
    final t = doc.lines[h.line].text;
    final indent = t.length - t.trimLeft().length;
    _move(Pos(h.line, h.col == indent ? 0 : indent), extend);
  }

  // ── Keyboard ────────────────────────────────────────────────────────────

  KeyEventResult _onKey(FocusNode node, KeyEvent e) {
    if (e is KeyUpEvent) return KeyEventResult.ignored;
    if (_composing.isValid && !_composing.isCollapsed) return KeyEventResult.skipRemainingHandlers; // the IME owns the keys
    final hw = HardwareKeyboard.instance;
    final ctrl = hw.isControlPressed || hw.isMetaPressed;
    final shift = hw.isShiftPressed;
    final alt = hw.isAltPressed;
    final k = e.logicalKey;
    final s = doc.sel;

    if (completion.open) {
      if (k == LogicalKeyboardKey.arrowDown) return _done(() => completion.step(1));
      if (k == LogicalKeyboardKey.arrowUp) return _done(() => completion.step(-1));
      if (k == LogicalKeyboardKey.pageDown) return _done(() => completion.step(8));
      if (k == LogicalKeyboardKey.pageUp) return _done(() => completion.step(-8));
      if (k == LogicalKeyboardKey.enter || k == LogicalKeyboardKey.tab || k == LogicalKeyboardKey.numpadEnter) {
        return _done(_acceptCompletion);
      }
      if (k == LogicalKeyboardKey.escape) return _done(completion.close);
    }

    if (ctrl && !alt) {
      switch (k) {
        case LogicalKeyboardKey.keyA:
          return _done(selectAll);
        case LogicalKeyboardKey.keyC:
          return _done(copy);
        case LogicalKeyboardKey.keyX:
          return _done(() => copy(cut: true));
        case LogicalKeyboardKey.keyV:
          return _done(paste);
        case LogicalKeyboardKey.keyZ:
          return _done(shift ? doc.redo : doc.undo);
        case LogicalKeyboardKey.keyY:
          return _done(doc.redo);
        case LogicalKeyboardKey.slash:
          return _done(toggleComment);
        case LogicalKeyboardKey.keyF:
          return _done(() => _openFind(replace: false));
        case LogicalKeyboardKey.keyH:
          return _done(() => _openFind(replace: true));
        case LogicalKeyboardKey.keyL:
          final (f, l) = _selLines();
          return _done(() => doc.select(Sel(Pos(f, 0), l + 1 < doc.lineCount ? Pos(l + 1, 0) : Pos(l, doc.lines[l].text.length))));
        case LogicalKeyboardKey.keyK when shift:
          return _done(_deleteLines);
        case LogicalKeyboardKey.space:
          return _done(() => _complete(explicit: true));
        case LogicalKeyboardKey.arrowLeft:
          return _done(() => _move(doc.wordLeft(s.head), shift));
        case LogicalKeyboardKey.arrowRight:
          return _done(() => _move(doc.wordRight(s.head), shift));
        case LogicalKeyboardKey.arrowUp:
          return _done(() => _v.jumpTo(math.max(0, _v.offset - _lh)));
        case LogicalKeyboardKey.arrowDown:
          return _done(() => _v.jumpTo(math.min(_v.position.maxScrollExtent, _v.offset + _lh)));
        case LogicalKeyboardKey.home:
          return _done(() => _home(shift, true));
        case LogicalKeyboardKey.end:
          return _done(() => _move(doc.endPos, shift));
        case LogicalKeyboardKey.backspace:
          return _done(() => _backspace(word: true));
        case LogicalKeyboardKey.delete:
          return _done(() => _delete(word: true));
        default:
          return KeyEventResult.ignored; // app shortcuts (save, compile, …)
      }
    }
    if (alt && !ctrl) {
      if (k == LogicalKeyboardKey.arrowUp) return _done(() => _moveLines(-1));
      if (k == LogicalKeyboardKey.arrowDown) return _done(() => _moveLines(1));
    }
    switch (k) {
      case LogicalKeyboardKey.arrowLeft:
        final h = s.head;
        if (!shift && !s.isEmpty) return _done(() => _move(s.start, false));
        final to = h.col > 0
            ? Pos(h.line, doc.stepLeft(doc.lines[h.line].text, h.col))
            : (h.line > 0 ? Pos(h.line - 1, doc.lines[h.line - 1].text.length) : h);
        return _done(() => _move(to, shift));
      case LogicalKeyboardKey.arrowRight:
        final h = s.head;
        if (!shift && !s.isEmpty) return _done(() => _move(s.end, false));
        final t = doc.lines[h.line].text;
        final to = h.col < t.length ? Pos(h.line, doc.stepRight(t, h.col)) : (h.line + 1 < doc.lineCount ? Pos(h.line + 1, 0) : h);
        return _done(() => _move(to, shift));
      case LogicalKeyboardKey.arrowUp:
        return _done(() => _vertical(-1, shift));
      case LogicalKeyboardKey.arrowDown:
        return _done(() => _vertical(1, shift));
      case LogicalKeyboardKey.pageUp:
        return _done(() => _vertical(-(_viewportH / _lh).floor() + 1, shift));
      case LogicalKeyboardKey.pageDown:
        return _done(() => _vertical((_viewportH / _lh).floor() - 1, shift));
      case LogicalKeyboardKey.home:
        return _done(() => _home(shift, false));
      case LogicalKeyboardKey.end:
        return _done(() => _move(Pos(s.head.line, doc.lines[s.head.line].text.length), shift));
      case LogicalKeyboardKey.backspace:
        return _done(_backspace);
      case LogicalKeyboardKey.delete:
        return _done(_delete);
      case LogicalKeyboardKey.enter || LogicalKeyboardKey.numpadEnter:
        if (shift || alt) return KeyEventResult.ignored;
        return _done(_newline);
      case LogicalKeyboardKey.tab:
        return _done(() => _indent(shift));
      case LogicalKeyboardKey.escape:
        if (find.open) return _done(() => find.close());
        if (!s.isEmpty) return _done(() => doc.select(Sel.at(s.head)));
        return KeyEventResult.ignored; // e.g. stop a running build
      case LogicalKeyboardKey.f3:
        return _done(() => _findStep(shift ? -1 : 1));
    }
    // Printable characters: let the platform's text input insert them.
    return e.character != null && !alt ? KeyEventResult.skipRemainingHandlers : KeyEventResult.ignored;
  }

  KeyEventResult _done(FutureOr<void> Function() f) {
    f();
    _restartBlink();
    return KeyEventResult.handled;
  }

  // ── Find / replace ──────────────────────────────────────────────────────

  void _openFind({required bool replace}) {
    final s = doc.sel;
    final seed = !s.isEmpty && s.start.line == s.end.line ? doc.selectedText : null;
    find.show(doc, seed: seed, replace: replace);
  }

  void _findStep(int dir) {
    final m = find.step(doc, dir);
    if (m != null) {
      doc.select(Sel(m.$1, m.$2));
      _ensureVisible(m.$2, center: true);
    }
  }

  void replaceCurrent() {
    final m = find.current(doc);
    if (m == null) return _findStep(1);
    doc.replace(m.$1, m.$2, find.replacementFor(doc, m), kind: 'edit');
    _findStep(1);
  }

  void replaceAll() {
    final n = find.replaceAll(doc);
    if (n > 0) find.status = 'Replaced $n';
  }

  // ── Completion ──────────────────────────────────────────────────────────

  void _completeAfterTyping() => _complete(explicit: false);

  void _complete({required bool explicit}) {
    final h = doc.sel.head;
    if (!doc.sel.isEmpty) return completion.close();
    final before = doc.lines[h.line].text.substring(0, h.col);
    final ctx = completionContext(doc, before, widget.source, explicit: explicit);
    if (ctx == null || ctx.items.isEmpty) return completion.close();
    completion.show(Pos(h.line, h.col - ctx.prefix.length), ctx);
  }

  void _acceptCompletion() {
    final item = completion.selected;
    final anchor = completion.anchor;
    completion.close();
    if (item == null || anchor == null) return;
    final h = doc.sel.head;
    final t = doc.lines[h.line].text;
    var end = h;
    // Replace the rest of the word too (completing in the middle of `\ref{fi|g:x}`).
    var j = h.col;
    while (j < t.length && RegExp(r'[\w:.\-@*]').hasMatch(t[j])) {
      j++;
    }
    end = Pos(h.line, j);
    var insert = item.insert;
    var caret = item.caret;
    // An auto-closed `}` after the cursor is already there.
    if ((item.closeBrace || item.envBody) && j < t.length && t[j] == '}') end = Pos(h.line, j + 1);
    if (item.envBody) {
      final line = doc.lines[h.line].text;
      final indent = RegExp(r'^[ \t]*').stringMatch(line) ?? '';
      final (body, at) = item.body ?? ('  ', 2);
      final head = '${item.insert}}\n$indent';
      final bodyIndented = body.replaceAll('\n', '\n$indent');
      insert = '$head$bodyIndented\n$indent\\end{${item.insert}}';
      caret = head.length + (at < 0 ? bodyIndented.length : at + indent.length * '\n'.allMatches(body.substring(0, at)).length);
    }
    final start = anchor;
    doc.replace(start, end, insert, kind: 'edit');
    doc.select(Sel.at(_offsetPos(start, insert, caret)));
    doc.breakUndoGroup();
    _syncInput();
    _ensureVisible(doc.sel.head);
    // `\begin{` → offer the environments right away.
    if (insert.endsWith('{}') && caret == insert.length - 1) _complete(explicit: true);
  }

  static Pos _offsetPos(Pos start, String text, int offset) {
    final pre = text.substring(0, offset.clamp(0, text.length));
    final nl = pre.lastIndexOf('\n');
    if (nl < 0) return Pos(start.line, start.col + pre.length);
    return Pos(start.line + '\n'.allMatches(pre).length, pre.length - nl - 1);
  }

  // ── Mouse ───────────────────────────────────────────────────────────────

  void _pointerDown(PointerDownEvent e) {
    widget.focusNode.requestFocus();
    completion.close();
    if (e.buttons == kSecondaryMouseButton) {
      _contextMenu(e.position, _posAt(e.localPosition));
      return;
    }
    if (e.buttons != kPrimaryMouseButton) return;
    final now = DateTime.now();
    final near = (e.localPosition - _lastDownPos).distance < 6;
    _clicks = (now.difference(_lastDown) < const Duration(milliseconds: 450) && near) ? _clicks % 3 + 1 : 1;
    _lastDown = now;
    _lastDownPos = e.localPosition;
    final p = _posAt(e.localPosition);
    _goalX = null;
    doc.breakUndoGroup();
    if (HardwareKeyboard.instance.isShiftPressed && _clicks == 1) {
      doc.select(Sel(doc.sel.anchor, p));
      _dragOrigin = Sel.at(doc.sel.anchor);
    } else if (_clicks == 2) {
      _dragOrigin = doc.wordAt(p);
      doc.select(_dragOrigin!);
    } else if (_clicks == 3) {
      _dragOrigin = Sel(Pos(p.line, 0), p.line + 1 < doc.lineCount ? Pos(p.line + 1, 0) : Pos(p.line, doc.lines[p.line].text.length));
      doc.select(_dragOrigin!);
    } else {
      _dragOrigin = Sel.at(p);
      doc.select(Sel.at(p));
    }
    _restartBlink();
  }

  void _pointerMove(PointerMoveEvent e) {
    if (_dragOrigin == null || e.buttons != kPrimaryMouseButton) return;
    _extendDrag(e.localPosition);
    // Auto-scroll while dragging outside the visible area.
    final vy = e.localPosition.dy - _v.offset;
    final vx = e.localPosition.dx - _h.offset;
    _dragViewport = Offset(vx, vy);
    if (vy < 0 || vy > _viewportH || vx < 0 || vx > _viewportW) {
      _autoScroll ??= Timer.periodic(const Duration(milliseconds: 30), (_) => _autoScrollTick());
    } else {
      _autoScroll?.cancel();
      _autoScroll = null;
    }
  }

  void _autoScrollTick() {
    final d = _dragViewport;
    if (d == null || _dragOrigin == null) return;
    double over(double v, double max) => v < 0 ? v : (v > max ? v - max : 0);
    final dy = over(d.dy, _viewportH), dx = over(d.dx, _viewportW);
    if (dy != 0) _v.jumpTo((_v.offset + dy.clamp(-60, 60)).clamp(0, _v.position.maxScrollExtent));
    if (dx != 0) _h.jumpTo((_h.offset + dx.clamp(-40, 40)).clamp(0, _h.position.maxScrollExtent));
    _extendDrag(Offset(d.dx + _h.offset, d.dy + _v.offset));
  }

  void _extendDrag(Offset local) {
    final o = _dragOrigin!;
    var p = _posAt(local);
    if (_clicks == 2) {
      final w = doc.wordAt(p);
      p = p < o.start ? w.start : w.end;
    } else if (_clicks == 3) {
      p = p < o.start ? Pos(p.line, 0) : (p.line + 1 < doc.lineCount ? Pos(p.line + 1, 0) : Pos(p.line, doc.lines[p.line].text.length));
    }
    final anchor = p < o.start ? o.end : o.start;
    doc.select(Sel(anchor, p));
  }

  void _pointerUp(PointerEvent e) {
    _dragOrigin = null;
    _autoScroll?.cancel();
    _autoScroll = null;
    _syncInput();
  }

  Future<void> _contextMenu(Offset global, Pos at) async {
    final s = doc.sel;
    if (s.isEmpty || at < s.start || at > s.end) doc.select(Sel.at(at));
    final r = await showMenu<String>(
      context: context,
      position: RelativeRect.fromLTRB(global.dx, global.dy, global.dx, global.dy),
      items: [
        PopupMenuItem(value: 'cut', height: 32, enabled: !doc.sel.isEmpty, child: const Text('Cut')),
        PopupMenuItem(value: 'copy', height: 32, enabled: !doc.sel.isEmpty, child: const Text('Copy')),
        const PopupMenuItem(value: 'paste', height: 32, child: Text('Paste')),
        const PopupMenuDivider(),
        const PopupMenuItem(value: 'all', height: 32, child: Text('Select all')),
        const PopupMenuItem(value: 'comment', height: 32, child: Text('Toggle comment')),
        const PopupMenuItem(value: 'find', height: 32, child: Text('Find / replace')),
      ],
    );
    switch (r) {
      case 'cut':
        await copy(cut: true);
      case 'copy':
        await copy();
      case 'paste':
        await paste();
      case 'all':
        selectAll();
      case 'comment':
        toggleComment();
      case 'find':
        _openFind(replace: true);
    }
    widget.focusNode.requestFocus();
  }

  // ── Build ───────────────────────────────────────────────────────────────

  @override
  Widget build(BuildContext context) {
    final k = KT.of(context);
    _metrics(k);
    final lines = doc.lineCount;
    final gutterW = math.max(3, '$lines'.length) * _charW + 30;
    final contentH = _padTop * 2 + lines * _lh;

    final editor = LayoutBuilder(
      builder: (context, box) {
        _viewportH = box.maxHeight;
        _viewportW = math.max(100, box.maxWidth - gutterW);
        final contentW = math.max(_viewportW, doc.maxLineLength * _charW + _padLeft * 2 + 80);
        final h = math.max(contentH, box.maxHeight);
        return Stack(
          children: [
            Scrollbar(
              controller: _v,
              child: SingleChildScrollView(
                controller: _v,
                child: SizedBox(
                  height: h,
                  child: Row(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      RepaintBoundary(
                        child: CustomPaint(size: Size(gutterW, h), painter: _GutterPainter(this, k)),
                      ),
                      Expanded(
                        child: Scrollbar(
                          controller: _h,
                          notificationPredicate: (n) => n.depth == 0,
                          child: SingleChildScrollView(
                            controller: _h,
                            scrollDirection: Axis.horizontal,
                            child: MouseRegion(
                              cursor: SystemMouseCursors.text,
                              child: Listener(
                                onPointerDown: _pointerDown,
                                onPointerMove: _pointerMove,
                                onPointerUp: _pointerUp,
                                onPointerCancel: _pointerUp,
                                child: SizedBox(
                                  key: _contentKey,
                                  width: contentW,
                                  height: h,
                                  child: RepaintBoundary(
                                    child: CustomPaint(painter: _TextPainter(this, k), foregroundPainter: _CaretPainter(this, k)),
                                  ),
                                ),
                              ),
                            ),
                          ),
                        ),
                      ),
                    ],
                  ),
                ),
              ),
            ),
            if (completion.open) _completionPopup(k, gutterW),
          ],
        );
      },
    );

    return Focus(
      focusNode: widget.focusNode,
      onKeyEvent: _onKey,
      child: Semantics(
        textField: true,
        multiline: true,
        label: 'Source editor',
        value: doc.lines[doc.sel.head.line].text,
        child: ColoredBox(
          color: k.editorBg,
          child: Column(
            children: [
              if (find.open)
                FindBar(
                  state: find,
                  onNext: () => _findStep(1),
                  onPrev: () => _findStep(-1),
                  onReplace: replaceCurrent,
                  onReplaceAll: replaceAll,
                  onClose: () {
                    find.close();
                    widget.focusNode.requestFocus();
                  },
                  onQueryChanged: () {
                    find.recompute(doc);
                    final m = find.nearest(doc.sel.start);
                    if (m != null) {
                      find.index = m;
                      final r = find.matches[m];
                      _ensureVisible(r.$1, center: true);
                    }
                  },
                ),
              Expanded(child: editor),
            ],
          ),
        ),
      ),
    );
  }

  Widget _completionPopup(KT k, double gutterW) {
    final anchor = completion.anchor!;
    final r = caretRect(anchor);
    final left = gutterW + r.left - (_h.hasClients ? _h.offset : 0) - 8;
    final top = r.bottom - (_v.hasClients ? _v.offset : 0) + 2;
    final flipUp = top + 240 > _viewportH && top > 260;
    return Positioned(
      left: left.clamp(0, math.max(0, gutterW + _viewportW - 420)),
      top: flipUp ? null : top,
      bottom: flipUp ? _viewportH - (r.top - (_v.hasClients ? _v.offset : 0)) + 2 : null,
      child: CompletionPopup(
        state: completion,
        onPick: (i) {
          completion.index = i;
          _acceptCompletion();
          widget.focusNode.requestFocus();
        },
      ),
    );
  }
}

// ── Painters ──────────────────────────────────────────────────────────────

(int, int) _visible(EditorViewState s) {
  final top = s._v.hasClients ? s._v.offset : 0.0;
  final first = math.max(0, ((top - _padTop) / s._lh).floor());
  final last = math.min(s.doc.lineCount - 1, ((top + s._viewportH) / s._lh).ceil());
  return (first, last);
}

class _TextPainter extends CustomPainter {
  _TextPainter(this.s, this.k) : super(repaint: Listenable.merge([s.doc, s._v, s.find]));
  final EditorViewState s;
  final KT k;

  @override
  void paint(Canvas canvas, Size size) {
    final doc = s.doc;
    final lh = s._lh;
    final (first, last) = _visible(s);
    final sel = doc.sel;
    final focused = s.widget.focusNode.hasFocus;
    final selPaint = Paint()..color = focused ? k.selection : k.selection.withValues(alpha: 0.55);
    double y(int l) => _padTop + l * lh;
    double x(int l, int c) => _padLeft + s.xOf(l, c);

    // Active line.
    if (sel.isEmpty && focused) {
      canvas.drawRect(Rect.fromLTWH(0, y(sel.head.line), size.width, lh), Paint()..color = k.activeLine);
    }
    // Search matches.
    if (s.find.open && s.find.matches.isNotEmpty) {
      final p = Paint()..color = k.warning.withValues(alpha: 0.22);
      final cur = Paint()..color = k.warning.withValues(alpha: 0.5);
      final ms = s.find.matches;
      for (var i = s.find.firstOnOrAfter(first); i < ms.length && ms[i].$1.line <= last; i++) {
        final (a, b) = ms[i];
        if (a.line != b.line) continue;
        canvas.drawRRect(
          RRect.fromRectAndRadius(
            Rect.fromLTRB(x(a.line, a.col), y(a.line) + 1, x(b.line, b.col), y(a.line) + lh - 1),
            const Radius.circular(2),
          ),
          i == s.find.index ? cur : p,
        );
      }
    }
    // Other occurrences of the selected word.
    if (!sel.isEmpty && sel.start.line == sel.end.line && !s.find.open) {
      final w = doc.selectedText;
      if (w.length >= 2 && w.length <= 80 && w.trim() == w && RegExp(r'^[\w\\@:.-]+$').hasMatch(w)) {
        final p = Paint()..color = k.accentSoft;
        for (var l = first; l <= last; l++) {
          final t = doc.lines[l].text;
          for (var i = t.indexOf(w); i >= 0; i = t.indexOf(w, i + w.length)) {
            if (l == sel.start.line && i == sel.start.col) continue;
            canvas.drawRRect(
              RRect.fromRectAndRadius(Rect.fromLTRB(x(l, i), y(l) + 1, x(l, i + w.length), y(l) + lh - 1), const Radius.circular(2)),
              p,
            );
          }
        }
      }
    }
    // Selection.
    if (!sel.isEmpty) {
      final a = sel.start, b = sel.end;
      for (var l = math.max(first, a.line); l <= math.min(last, b.line); l++) {
        final x0 = l == a.line ? x(l, a.col) : _padLeft;
        final x1 = l == b.line ? x(l, b.col) : x(l, doc.lines[l].text.length) + s._charW * 0.6;
        canvas.drawRect(Rect.fromLTRB(x0, y(l), x1, y(l) + lh), selPaint);
      }
    }
    // Matching bracket.
    if (sel.isEmpty && focused) {
      final m = doc.matchingBracket(sel.head);
      if (m != null) {
        final p = Paint()..color = k.accent.withValues(alpha: 0.18);
        final border = Paint()
          ..color = k.accent.withValues(alpha: 0.55)
          ..style = PaintingStyle.stroke;
        for (final b in [m.$1, m.$2]) {
          if (b.line < first || b.line > last) continue;
          final r = Rect.fromLTRB(x(b.line, b.col), y(b.line) + 2, x(b.line, b.col + 1), y(b.line) + lh - 2);
          canvas
            ..drawRect(r, p)
            ..drawRect(r, border);
        }
      }
    }
    // Diagnostics: background for errors, wavy underline for all.
    for (final m in doc.marks) {
      if (m.end.line < first) continue;
      if (m.start.line > last) break;
      final c = switch (m.severity) {
        'warning' => k.warning,
        'info' => k.info,
        _ => k.danger,
      };
      for (var l = math.max(first, m.start.line); l <= math.min(last, m.end.line); l++) {
        final x0 = l == m.start.line ? x(l, m.start.col) : _padLeft;
        final x1 = l == m.end.line ? x(l, m.end.col) : x(l, doc.lines[l].text.length);
        if (m.severity == 'error') {
          canvas.drawRect(Rect.fromLTRB(x0, y(l) + 2, x1, y(l) + lh - 2), Paint()..color = k.dangerSoft.withValues(alpha: 0.7));
        }
        _wavy(canvas, x0, math.max(x1, x0 + s._charW), y(l) + lh - 3, c, dotted: m.severity == 'info');
      }
    }
    // Text.
    for (var l = first; l <= last; l++) {
      canvas.drawParagraph(s._para(l), Offset(_padLeft, y(l)));
    }
    // IME composition.
    final comp = s._composing;
    if (comp.isValid && !comp.isCollapsed && s._sentLine < doc.lineCount) {
      final l = s._sentLine;
      canvas.drawLine(
        Offset(x(l, comp.start), y(l) + lh - 2),
        Offset(x(l, comp.end), y(l) + lh - 2),
        Paint()
          ..color = k.text
          ..strokeWidth = 1.5,
      );
    }
  }

  void _wavy(Canvas c, double x0, double x1, double y, Color color, {bool dotted = false}) {
    final p = Paint()
      ..color = color
      ..style = PaintingStyle.stroke
      ..strokeWidth = 1.2;
    if (dotted) {
      for (var x = x0; x < x1; x += 4) {
        c.drawCircle(Offset(x + 1, y + 1), 0.9, p..style = PaintingStyle.fill);
      }
      return;
    }
    final path = Path()..moveTo(x0, y);
    var up = true;
    for (var x = x0; x < x1; x += 3) {
      path.lineTo(math.min(x + 3, x1), up ? y - 1.6 : y + 1.6);
      up = !up;
    }
    c.drawPath(path, p);
  }

  @override
  bool shouldRepaint(_TextPainter o) => true;
}

class _CaretPainter extends CustomPainter {
  _CaretPainter(this.s, this.k) : super(repaint: Listenable.merge([s._caretOn, s.doc]));
  final EditorViewState s;
  final KT k;

  @override
  void paint(Canvas canvas, Size size) {
    if (!s.widget.focusNode.hasFocus || !s._caretOn.value) return;
    final r = s.caretRect(s.doc.sel.head);
    canvas.drawRect(Rect.fromLTWH(r.left - 1, r.top + 2, 2, r.height - 4), Paint()..color = k.accent);
  }

  @override
  bool shouldRepaint(_CaretPainter o) => o.k != k;
}

class _GutterPainter extends CustomPainter {
  _GutterPainter(this.s, this.k) : super(repaint: Listenable.merge([s.doc, s._v]));
  final EditorViewState s;
  final KT k;

  @override
  void paint(Canvas canvas, Size size) {
    final (first, last) = _visible(s);
    final lh = s._lh;
    final doc = s.doc;
    final caretLine = doc.sel.head.line;
    final sev = <int, String>{};
    for (final m in doc.marks) {
      if (m.start.line < first || m.start.line > last) continue;
      if (sev[m.start.line] != 'error') sev[m.start.line] = m.severity;
    }
    final style = TextStyle(fontFamily: monoFamily, fontFamilyFallback: monoFallback, fontSize: s.widget.fontSize * 0.92, color: k.faint);
    // Same strut as the text, so numbers sit on the text's baseline.
    final strut = StrutStyle(
      fontFamily: monoFamily,
      fontFamilyFallback: monoFallback,
      fontSize: s.widget.fontSize,
      height: 1.6,
      forceStrutHeight: true,
    );
    for (var l = first; l <= last; l++) {
      final y = _padTop + l * lh;
      final m = sev[l];
      if (m != null) {
        final c = m == 'warning' ? k.warning : (m == 'info' ? k.info : k.danger);
        canvas.drawCircle(Offset(9, y + lh / 2), 3.5, Paint()..color = c);
      }
      final tp = TextPainter(
        text: TextSpan(
          text: '${l + 1}',
          style: l == caretLine ? style.copyWith(color: k.muted) : style,
        ),
        textDirection: TextDirection.ltr,
        textAlign: TextAlign.right,
        strutStyle: strut,
      )..layout(minWidth: size.width - 30, maxWidth: size.width - 30);
      tp.paint(canvas, Offset(18, y));
    }
  }

  @override
  bool shouldRepaint(_GutterPainter o) => o.k != k;
}
