// Find / replace: plain text, whole word or regular expression, with
// highlighted matches, "n of m", and replace-all as a single undo step.

import 'dart:async';

import 'package:flutter/services.dart';
import 'package:material_ui/material_ui.dart';

import '../theme.dart';
import 'document.dart';

class FindState extends ChangeNotifier {
  bool open = false;
  bool replaceMode = false;
  bool caseSensitive = false, regex = false, wholeWord = false;
  final query = TextEditingController();
  final replacement = TextEditingController();
  final queryFocus = FocusNode();
  List<(Pos, Pos)> matches = const [];
  int index = -1;
  String? status;
  RegExp? _re;
  Timer? _debounce;
  static const maxMatches = 20000;

  void show(EditorDocument doc, {String? seed, required bool replace}) {
    open = true;
    replaceMode = replace || replaceMode;
    if (seed != null && seed.isNotEmpty) query.text = seed;
    query.selection = TextSelection(baseOffset: 0, extentOffset: query.text.length);
    recompute(doc);
    index = nearest(doc.sel.start) ?? -1;
    notifyListeners();
    WidgetsBinding.instance.addPostFrameCallback((_) => queryFocus.requestFocus());
  }

  void close() {
    open = false;
    matches = const [];
    notifyListeners();
  }

  void docChanged(EditorDocument doc) {
    if (!open) return;
    _debounce?.cancel();
    _debounce = Timer(const Duration(milliseconds: 120), () => recompute(doc));
  }

  void recompute(EditorDocument doc) {
    status = null;
    final q = query.text;
    if (!open || q.isEmpty) {
      matches = const [];
      _re = null;
      notifyListeners();
      return;
    }
    try {
      var src = regex ? q : RegExp.escape(q);
      if (wholeWord) src = '(?<![\\w\\\\])$src(?!\\w)';
      _re = RegExp(src, caseSensitive: caseSensitive, unicode: true);
    } on FormatException catch (e) {
      _re = null;
      matches = const [];
      status = 'Invalid pattern: ${e.message}';
      notifyListeners();
      return;
    }
    final out = <(Pos, Pos)>[];
    final re = _re!;
    for (var l = 0; l < doc.lineCount && out.length < maxMatches; l++) {
      final t = doc.lines[l].text;
      if (t.isEmpty) continue;
      for (final m in re.allMatches(t)) {
        if (m.end == m.start) continue;
        out.add((Pos(l, m.start), Pos(l, m.end)));
        if (out.length >= maxMatches) break;
      }
    }
    matches = out;
    if (index >= matches.length) index = matches.isEmpty ? -1 : 0;
    notifyListeners();
  }

  /// Index of the first match on or after `line` (binary search).
  int firstOnOrAfter(int line) {
    var lo = 0, hi = matches.length;
    while (lo < hi) {
      final mid = (lo + hi) >> 1;
      if (matches[mid].$1.line < line) {
        lo = mid + 1;
      } else {
        hi = mid;
      }
    }
    return lo;
  }

  int? nearest(Pos p) {
    if (matches.isEmpty) return null;
    for (var i = firstOnOrAfter(p.line); i < matches.length; i++) {
      if (matches[i].$1 >= p) return i;
    }
    return 0;
  }

  /// The match the selection is on, if any.
  (Pos, Pos)? current(EditorDocument doc) {
    final s = doc.sel;
    for (final m in matches) {
      if (m.$1 == s.start && m.$2 == s.end) return m;
      if (m.$1 > s.end) break;
    }
    return null;
  }

  (Pos, Pos)? step(EditorDocument doc, int dir) {
    if (matches.isEmpty) return null;
    final s = doc.sel;
    final cur = matches.indexWhere((m) => m.$1 == s.start && m.$2 == s.end);
    if (cur >= 0) {
      index = (cur + dir) % matches.length;
    } else {
      final n = nearest(dir > 0 ? s.end : s.start) ?? 0;
      index = dir > 0 ? n : (n - 1) % matches.length;
    }
    notifyListeners();
    return matches[index];
  }

  String replacementFor(EditorDocument doc, (Pos, Pos) m) {
    final r = replacement.text;
    if (!regex || _re == null) return r;
    final t = doc.textIn(m.$1, m.$2);
    final match = _re!.firstMatch(t);
    if (match == null) return r;
    // $1 … $9 and $0 refer to groups.
    return r.replaceAllMapped(RegExp(r'\$(\d)'), (g) => match.group(int.parse(g.group(1)!)) ?? '');
  }

  /// Replaces every match in one edit (one undo step). Returns the count.
  int replaceAll(EditorDocument doc) {
    if (matches.isEmpty) return 0;
    final first = matches.first.$1.line, last = matches.last.$1.line;
    final out = <String>[];
    var mi = 0;
    for (var l = first; l <= last; l++) {
      final t = doc.lines[l].text;
      final b = StringBuffer();
      var at = 0;
      while (mi < matches.length && matches[mi].$1.line == l) {
        final m = matches[mi];
        b
          ..write(t.substring(at, m.$1.col))
          ..write(replacementFor(doc, m));
        at = m.$2.col;
        mi++;
      }
      b.write(t.substring(at));
      out.add(b.toString());
    }
    final n = matches.length;
    doc.replace(Pos(first, 0), Pos(last, doc.lines[last].text.length), out.join('\n'), kind: 'replaceAll');
    recompute(doc);
    return n;
  }

  void toggleReplace() {
    replaceMode = !replaceMode;
    notifyListeners();
  }

  @override
  void dispose() {
    _debounce?.cancel();
    query.dispose();
    replacement.dispose();
    queryFocus.dispose();
    super.dispose();
  }
}

class FindBar extends StatelessWidget {
  const FindBar({
    super.key,
    required this.state,
    required this.onNext,
    required this.onPrev,
    required this.onReplace,
    required this.onReplaceAll,
    required this.onClose,
    required this.onQueryChanged,
  });

  final FindState state;
  final VoidCallback onNext, onPrev, onReplace, onReplaceAll, onClose, onQueryChanged;

  @override
  Widget build(BuildContext context) {
    final k = KT.of(context);
    final s = state;
    final count =
        s.status ??
        (s.query.text.isEmpty
            ? ''
            : (s.matches.isEmpty
                  ? 'No results'
                  : '${s.index < 0 ? '?' : s.index + 1} of ${s.matches.length}${s.matches.length >= FindState.maxMatches ? '+' : ''}'));

    Widget toggle(String label, String tip, bool on, VoidCallback f) => Tooltip(
      message: tip,
      child: InkWell(
        borderRadius: BorderRadius.circular(4),
        onTap: f,
        child: Container(
          width: 26,
          height: 24,
          alignment: Alignment.center,
          decoration: BoxDecoration(
            color: on ? k.accentSoft : null,
            borderRadius: BorderRadius.circular(4),
            border: Border.all(color: on ? k.accent : Colors.transparent),
          ),
          child: Text(
            label,
            style: TextStyle(fontSize: 12, fontWeight: FontWeight.w600, color: on ? k.accent : k.muted),
          ),
        ),
      ),
    );

    Widget field(
      TextEditingController c,
      String hint, {
      FocusNode? focus,
      required VoidCallback onEnter,
      ValueChanged<String>? onChanged,
    }) => SizedBox(
      width: 260,
      height: 30,
      child: CallbackShortcuts(
        bindings: {
          const SingleActivator(LogicalKeyboardKey.escape): onClose,
          const SingleActivator(LogicalKeyboardKey.enter, shift: true): onPrev,
          const SingleActivator(LogicalKeyboardKey.f3): onNext,
          const SingleActivator(LogicalKeyboardKey.f3, shift: true): onPrev,
        },
        child: TextField(
          controller: c,
          focusNode: focus,
          onChanged: onChanged,
          onSubmitted: (_) {
            onEnter();
            focus?.requestFocus();
          },
          style: TextStyle(fontSize: 13, fontFamily: monoFamily, fontFamilyFallback: monoFallback, color: k.text),
          decoration: InputDecoration(hintText: hint, contentPadding: const EdgeInsets.symmetric(horizontal: 8, vertical: 7)),
        ),
      ),
    );

    Widget btn(IconData i, String tip, VoidCallback? f) => IconButton(
      icon: Icon(i, size: 16),
      tooltip: tip,
      onPressed: f,
      constraints: const BoxConstraints(minWidth: 28, minHeight: 28),
      padding: EdgeInsets.zero,
    );

    return Container(
      padding: const EdgeInsets.fromLTRB(8, 6, 8, 6),
      decoration: BoxDecoration(
        color: k.surface,
        border: Border(bottom: BorderSide(color: k.border)),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              btn(s.replaceMode ? Icons.expand_more : Icons.chevron_right, 'Toggle replace (Ctrl+H)', s.toggleReplace),
              field(s.query, 'Find', focus: s.queryFocus, onEnter: onNext, onChanged: (_) => onQueryChanged()),
              const SizedBox(width: 6),
              toggle('Aa', 'Match case', s.caseSensitive, () {
                s.caseSensitive = !s.caseSensitive;
                onQueryChanged();
              }),
              toggle('W', 'Whole word', s.wholeWord, () {
                s.wholeWord = !s.wholeWord;
                onQueryChanged();
              }),
              toggle('.*', r'Regular expression ($1 in the replacement refers to a group)', s.regex, () {
                s.regex = !s.regex;
                onQueryChanged();
              }),
              const SizedBox(width: 8),
              Flexible(
                child: Text(
                  count,
                  overflow: TextOverflow.ellipsis,
                  style: TextStyle(fontSize: 12, color: s.status != null ? k.danger : k.muted),
                ),
              ),
              btn(Icons.keyboard_arrow_up, 'Previous (Shift+Enter)', s.matches.isEmpty ? null : onPrev),
              btn(Icons.keyboard_arrow_down, 'Next (Enter)', s.matches.isEmpty ? null : onNext),
              const Spacer(),
              btn(Icons.close, 'Close (Esc)', onClose),
            ],
          ),
          if (s.replaceMode)
            Padding(
              padding: const EdgeInsets.only(top: 4, left: 28),
              child: Row(
                children: [
                  field(s.replacement, 'Replace', onEnter: onReplace),
                  const SizedBox(width: 6),
                  TextButton(onPressed: s.matches.isEmpty ? null : onReplace, child: const Text('Replace')),
                  TextButton(onPressed: s.matches.isEmpty ? null : onReplaceAll, child: const Text('Replace all')),
                ],
              ),
            ),
        ],
      ),
    );
  }
}
