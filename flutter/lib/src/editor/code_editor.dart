// The source editor: EditableText with a fixed line height, so the gutter and
// the active-line band are plain arithmetic (no text measuring), and the
// gutter paints only the visible line numbers.

import 'dart:math' as math;

import 'package:flutter/services.dart';
import 'package:material_ui/material_ui.dart';

import '../theme.dart';
import 'code_controller.dart';

const _padTop = 12.0;
const _padLeft = 14.0;

class CodeEditor extends StatefulWidget {
  const CodeEditor({super.key, required this.controller, required this.fontSize, required this.focusNode});

  final CodeController controller;
  final double fontSize;
  final FocusNode focusNode;

  @override
  State<CodeEditor> createState() => CodeEditorState();
}

class CodeEditorState extends State<CodeEditor> {
  final _v = ScrollController();
  final _h = ScrollController();
  final _textKey = GlobalKey<EditableTextState>();
  double _viewport = 600;

  double get lineHeight => (widget.fontSize * 1.6).roundToDouble();

  double _charWidth = 8;
  double _measuredFor = 0;

  void _measure() {
    if (_measuredFor == widget.fontSize) return;
    final tp = TextPainter(
      text: TextSpan(text: 'M' * 20, style: _style(null)),
      textDirection: TextDirection.ltr,
    )..layout();
    _charWidth = tp.width / 20;
    _measuredFor = widget.fontSize;
  }

  TextStyle _style(KT? k) =>
      TextStyle(fontFamily: monoFamily, fontFamilyFallback: monoFallback, fontSize: widget.fontSize, height: 1.6, color: k?.text);

  @override
  void initState() {
    super.initState();
    widget.controller.addListener(_changed);
  }

  @override
  void didUpdateWidget(CodeEditor oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.controller != widget.controller) {
      oldWidget.controller.removeListener(_changed);
      widget.controller.addListener(_changed);
    }
  }

  @override
  void dispose() {
    widget.controller.removeListener(_changed);
    _v.dispose();
    _h.dispose();
    super.dispose();
  }

  void _changed() => setState(() {});

  /// Puts the caret at 1-based `line`/`column` and scrolls it to the upper third.
  void reveal(int line, [int? column]) {
    final c = widget.controller;
    final off = c.offsetOf(line, column ?? 1);
    c.selection = TextSelection.collapsed(offset: off);
    widget.focusNode.requestFocus();
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!_v.hasClients) return;
      final y = _padTop + (line - 1) * lineHeight - _viewport / 3;
      _v.animateTo(y.clamp(0, _v.position.maxScrollExtent), duration: const Duration(milliseconds: 180), curve: Curves.easeOutCubic);
    });
  }

  // ── Editing helpers ─────────────────────────────────────────────────────

  void _replace(int start, int end, String text, {int? caret}) {
    final c = widget.controller;
    c.value = c.value.copyWith(
      text: c.text.replaceRange(start, end, text),
      selection: TextSelection.collapsed(offset: caret ?? start + text.length),
      composing: TextRange.empty,
    );
  }

  void _indent(bool outdent) {
    final c = widget.controller;
    final sel = c.selection;
    if (!sel.isValid) return;
    if (sel.isCollapsed && !outdent) {
      _replace(sel.start, sel.end, '  ');
      return;
    }
    final first = c.lineOf(sel.start), last = c.lineOf(math.max(sel.start, sel.end - 1));
    final ls = c.lineStarts;
    final buf = StringBuffer();
    final t = c.text;
    final blockStart = ls[first];
    final blockEnd = last + 1 < ls.length ? ls[last + 1] : t.length;
    for (var l = first; l <= last; l++) {
      final end = l + 1 < ls.length ? ls[l + 1] : t.length;
      var line = t.substring(ls[l], end);
      if (outdent) {
        line = line.startsWith('  ') ? line.substring(2) : line.replaceFirst(RegExp(r'^[ \t]'), '');
      } else {
        line = '  $line';
      }
      buf.write(line);
    }
    final out = buf.toString();
    c.value = c.value.copyWith(
      text: t.replaceRange(blockStart, blockEnd, out),
      selection: TextSelection(baseOffset: blockStart, extentOffset: blockStart + out.length - (out.endsWith('\n') ? 1 : 0)),
      composing: TextRange.empty,
    );
  }

  void _newline() {
    final c = widget.controller;
    final sel = c.selection;
    if (!sel.isValid) return;
    final ls = c.lineStarts;
    final lineStart = ls[c.lineOf(sel.start)];
    final indent = RegExp(r'^[ \t]*').stringMatch(c.text.substring(lineStart, sel.start)) ?? '';
    _replace(sel.start, sel.end, '\n$indent');
  }

  void _toggleComment() {
    final c = widget.controller;
    final sel = c.selection;
    if (!sel.isValid) return;
    final mark = c.lang == Lang.typst ? '// ' : '% ';
    final first = c.lineOf(sel.start), last = c.lineOf(math.max(sel.start, sel.end - (sel.isCollapsed ? 0 : 1)));
    final ls = c.lineStarts;
    final t = c.text;
    final start = ls[first];
    final end = last + 1 < ls.length ? ls[last + 1] - 1 : t.length;
    final lines = t.substring(start, end).split('\n');
    final bare = mark.trim();
    final allCommented = lines.where((l) => l.trim().isNotEmpty).every((l) => l.trimLeft().startsWith(bare));
    final out = lines
        .map((l) {
          if (l.trim().isEmpty) return l;
          if (allCommented) {
            final i = l.indexOf(bare);
            final cut = l.startsWith(mark, i) ? mark.length : bare.length;
            return l.substring(0, i) + l.substring(i + cut);
          }
          return mark + l;
        })
        .join('\n');
    c.value = c.value.copyWith(
      text: t.replaceRange(start, end, out),
      selection: sel.isCollapsed
          ? TextSelection.collapsed(offset: (sel.start + out.length - (end - start)).clamp(start, start + out.length))
          : TextSelection(baseOffset: start, extentOffset: start + out.length),
      composing: TextRange.empty,
    );
  }

  @override
  Widget build(BuildContext context) {
    final k = KT.of(context);
    _measure();
    final c = widget.controller;
    final lh = lineHeight;
    final lines = c.lineCount;
    final gutterW = math.max(3, '$lines'.length) * _charWidth + 28;
    final caretLine = c.selection.isValid ? c.lineOf(c.selection.extentOffset) : -1;
    final errorLines = <int, String>{};
    for (final m in c.marks) {
      final l = c.lineOf(m.start);
      if (errorLines[l] != 'error') errorLines[l] = m.severity;
    }
    final contentH = _padTop * 2 + lines * lh;

    return ColoredBox(
      color: k.editorBg,
      child: LayoutBuilder(
        builder: (context, box) {
          _viewport = box.maxHeight;
          final textW = math.max(box.maxWidth - gutterW, (c.maxLineLength + 4) * _charWidth + _padLeft * 2);
          return Scrollbar(
            controller: _v,
            child: SingleChildScrollView(
              controller: _v,
              child: SizedBox(
                height: math.max(contentH, box.maxHeight),
                child: Row(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    RepaintBoundary(
                      child: CustomPaint(
                        size: Size(gutterW, math.max(contentH, box.maxHeight)),
                        painter: _Gutter(_v, lines, lh, caretLine, errorLines, k, _style(k).copyWith(color: k.faint), box.maxHeight),
                      ),
                    ),
                    Expanded(
                      child: Scrollbar(
                        controller: _h,
                        child: SingleChildScrollView(
                          controller: _h,
                          scrollDirection: Axis.horizontal,
                          child: CustomPaint(
                            painter: _ActiveLine(caretLine, lh, k.activeLine),
                            child: SizedBox(
                              width: textW,
                              child: Padding(padding: const EdgeInsets.fromLTRB(_padLeft, _padTop, _padLeft, _padTop), child: _editable(k)),
                            ),
                          ),
                        ),
                      ),
                    ),
                  ],
                ),
              ),
            ),
          );
        },
      ),
    );
  }

  Widget _editable(KT k) {
    final style = _style(k);
    return Shortcuts(
      shortcuts: const {
        SingleActivator(LogicalKeyboardKey.tab): _IndentIntent(false),
        SingleActivator(LogicalKeyboardKey.tab, shift: true): _IndentIntent(true),
        SingleActivator(LogicalKeyboardKey.enter): _NewlineIntent(),
        SingleActivator(LogicalKeyboardKey.slash, control: true): _CommentIntent(),
        SingleActivator(LogicalKeyboardKey.slash, meta: true): _CommentIntent(),
      },
      child: Actions(
        actions: {
          _IndentIntent: CallbackAction<_IndentIntent>(onInvoke: (i) => _indent(i.outdent)),
          _NewlineIntent: CallbackAction<_NewlineIntent>(onInvoke: (_) => _newline()),
          _CommentIntent: CallbackAction<_CommentIntent>(onInvoke: (_) => _toggleComment()),
        },
        child: EditableText(
          key: _textKey,
          controller: widget.controller,
          focusNode: widget.focusNode,
          style: style,
          strutStyle: StrutStyle(
            fontFamily: monoFamily,
            fontFamilyFallback: monoFallback,
            fontSize: widget.fontSize,
            height: 1.6,
            forceStrutHeight: true,
          ),
          cursorColor: k.accent,
          cursorWidth: 2,
          backgroundCursorColor: k.faint,
          selectionColor: k.selection,
          maxLines: null,
          keyboardType: TextInputType.multiline,
          textInputAction: TextInputAction.newline,
          autocorrect: false,
          enableSuggestions: false,
          smartDashesType: SmartDashesType.disabled,
          smartQuotesType: SmartQuotesType.disabled,
          selectionControls: desktopTextSelectionHandleControls,
          contextMenuBuilder: (context, state) => AdaptiveTextSelectionToolbar.editableText(editableTextState: state),
          rendererIgnoresPointer: false,
        ),
      ),
    );
  }
}

class _IndentIntent extends Intent {
  const _IndentIntent(this.outdent);
  final bool outdent;
}

class _NewlineIntent extends Intent {
  const _NewlineIntent();
}

class _CommentIntent extends Intent {
  const _CommentIntent();
}

class _ActiveLine extends CustomPainter {
  _ActiveLine(this.line, this.lh, this.color);
  final int line;
  final double lh;
  final Color color;

  @override
  void paint(Canvas canvas, Size size) {
    if (line < 0) return;
    canvas.drawRect(Rect.fromLTWH(0, _padTop + line * lh, size.width, lh), Paint()..color = color);
  }

  @override
  bool shouldRepaint(_ActiveLine o) => o.line != line || o.lh != lh || o.color != color;
}

class _Gutter extends CustomPainter {
  _Gutter(this.scroll, this.lines, this.lh, this.caretLine, this.marks, this.k, this.style, this.viewport) : super(repaint: scroll);
  final ScrollController scroll;
  final int lines;
  final double lh;
  final int caretLine;
  final Map<int, String> marks;
  final KT k;
  final TextStyle style;
  final double viewport;

  @override
  void paint(Canvas canvas, Size size) {
    final top = scroll.hasClients ? scroll.offset : 0.0;
    final first = math.max(0, ((top - _padTop) / lh).floor());
    final last = math.min(lines - 1, ((top + viewport) / lh).ceil());
    for (var l = first; l <= last; l++) {
      final y = _padTop + l * lh;
      final sev = marks[l];
      if (sev != null) {
        final c = sev == 'warning' ? k.warning : (sev == 'info' ? k.info : k.danger);
        canvas.drawCircle(Offset(9, y + lh / 2), 3.5, Paint()..color = c);
      }
      final tp = TextPainter(
        text: TextSpan(
          text: '${l + 1}',
          style: l == caretLine ? style.copyWith(color: k.muted) : style,
        ),
        textDirection: TextDirection.ltr,
        textAlign: TextAlign.right,
      )..layout(minWidth: size.width - 30, maxWidth: size.width - 30);
      tp.paint(canvas, Offset(18, y));
    }
  }

  @override
  bool shouldRepaint(_Gutter o) =>
      o.lines != lines || o.caretLine != caretLine || o.marks.length != marks.length || o.k != k || o.lh != lh || o.viewport != viewport;
}
