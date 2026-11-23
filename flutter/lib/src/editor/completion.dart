// Context-aware completion for LaTeX and Typst: commands, environments with
// bodies, \ref/\cite and @ targets from the whole project, packages and files.

import 'dart:math' as math;

import 'package:material_ui/material_ui.dart';

import '../theme.dart';
import 'document.dart';
import 'highlight.dart';
import 'vocabulary.dart';

/// Project knowledge the editor completes from; filled by the app.
class CompletionSource {
  /// (label name, where it is defined)
  List<(String, String)> labels = const [];

  /// (bib key, title or entry type)
  List<(String, String)> citations = const [];

  /// Project files, relative paths with forward slashes.
  List<String> files = const [];
}

enum CompletionKind { command, environment, label, citation, package, file, function }

class CompletionItem {
  CompletionItem(this.label, this.detail, this.kind, {String? insert, int? caret, this.closeBrace = false, this.envBody = false, this.body})
    : insert = insert ?? label,
      caret = caret ?? (insert ?? label).length;

  final String label, detail;
  final CompletionKind kind;

  /// Replaces the typed prefix; the caret lands at `caret` within it.
  final String insert;
  final int caret;

  /// Also consume an auto-inserted `}` right after the cursor.
  final bool closeBrace;

  /// A `\begin{…}` completion: adds the body and the matching `\end{…}`.
  final bool envBody;
  final (String, int)? body;
}

class CompletionContext {
  CompletionContext(this.prefix, this.items);
  final String prefix;
  final List<CompletionItem> items;
}

final _cite = RegExp(
  r'\\(?:cite|citep|citet|parencite|textcite|autocite|footcite|citeauthor|citeyear|nocite|Cite|Parencite|Textcite)\*?(?:\[[^\]]*\])*\{(?:[^}]*,\s*)?([^},\s]*)$',
);
final _ref = RegExp(r'\\(?:ref|eqref|pageref|autoref|cref|Cref|nameref|vref|labelcref)\{(?:[^}]*,\s*)?([^},\s]*)$');
final _pkg = RegExp(r'\\(?:usepackage|RequirePackage)(?:\[[^\]]*\])?\{(?:[^}]*,\s*)?([^},\s]*)$');
final _file = RegExp(r'\\(input|include|includegraphics|addbibresource|bibliography|includepdf|lstinputlisting)(?:\[[^\]]*\])?\{([^}]*)$');
final _begin = RegExp(r'\\begin\{([^}]*)$');
final _cmd = RegExp(r'(?<!\\)\\([a-zA-Z@]*)$');
final _newcmd = RegExp(r'\\(?:newcommand|renewcommand|DeclareMathOperator|def)\*?\s*\{?\\([a-zA-Z@]+)');
final _typFn = RegExp(r'(?<!\\)#([a-zA-Z][\w.-]*)?$');
final _typRef = RegExp(r'(?<![\w\\])@([\w:.\-]*)$');
final _typFile = RegExp(r'#(?:image|include|import|bibliography|read|csv|json|yaml|toml)\(?\s*"([^"]*)$');

const _images = {'png', 'jpg', 'jpeg', 'pdf', 'eps', 'svg', 'gif', 'webp'};

String _ext(String f) {
  final i = f.lastIndexOf('.');
  return i < 0 ? '' : f.substring(i + 1).toLowerCase();
}

String _stripExt(String f) {
  final i = f.lastIndexOf('.');
  return i < 0 ? f : f.substring(0, i);
}

final _userCommands = Expando<(int, List<String>)>();

/// Commands defined in this document with \newcommand & co (cached per version).
List<String> _defined(EditorDocument doc) {
  final c = _userCommands[doc];
  if (c != null && c.$1 == doc.version) return c.$2;
  final out = <String>{};
  for (final l in doc.lines) {
    if (!l.text.contains('command') && !l.text.contains('\\def') && !l.text.contains('Operator')) continue;
    for (final m in _newcmd.allMatches(l.text)) {
      out.add(m.group(1)!);
    }
  }
  final list = out.toList()..sort();
  _userCommands[doc] = (doc.version, list);
  return list;
}

List<CompletionItem> _filter(Iterable<CompletionItem> items, String prefix) {
  final p = prefix.toLowerCase();
  final starts = <CompletionItem>[], contains = <CompletionItem>[];
  final seen = <String>{};
  for (final i in items) {
    if (!seen.add(i.label)) continue;
    final l = i.label.toLowerCase();
    if (l.startsWith(p)) {
      starts.add(i);
    } else if (p.length >= 2 && l.contains(p)) {
      contains.add(i);
    }
    if (starts.length > 300) break;
  }
  return [...starts, ...contains];
}

/// What to offer for the text left of the caret, or null for nothing.
CompletionContext? completionContext(EditorDocument doc, String before, CompletionSource? src, {required bool explicit}) {
  final s = src ?? CompletionSource();
  CompletionContext? ctx(String prefix, Iterable<CompletionItem> items) {
    final f = _filter(items, prefix);
    if (f.isEmpty) return null;
    if (!explicit && f.length == 1 && f.first.label == prefix) return null;
    return CompletionContext(prefix, f);
  }

  if (doc.lang == Lang.latex) {
    if (_cite.firstMatch(before) case final m?) {
      return ctx(m.group(1)!, [for (final (k, d) in s.citations) CompletionItem(k, d, CompletionKind.citation, closeBrace: false)]);
    }
    if (_ref.firstMatch(before) case final m?) {
      return ctx(m.group(1)!, [for (final (k, d) in s.labels) CompletionItem(k, d, CompletionKind.label)]);
    }
    if (_pkg.firstMatch(before) case final m?) {
      return ctx(m.group(1)!, [for (final e in packageInfo.entries) CompletionItem(e.key, e.value, CompletionKind.package)]);
    }
    if (_file.firstMatch(before) case final m?) {
      final cmd = m.group(1)!;
      final want = switch (cmd) {
        'includegraphics' => (String f) => _images.contains(_ext(f)),
        'includepdf' => (String f) => _ext(f) == 'pdf',
        'addbibresource' || 'bibliography' => (String f) => _ext(f) == 'bib',
        'lstinputlisting' => (String f) => true,
        _ => (String f) => _ext(f) == 'tex',
      };
      final dropExt = cmd == 'include' || cmd == 'bibliography';
      return ctx(m.group(2)!, [
        for (final f in s.files)
          if (want(f)) CompletionItem(dropExt ? _stripExt(f) : f, cmd == 'includegraphics' ? 'image' : 'file', CompletionKind.file),
      ]);
    }
    if (_begin.firstMatch(before) case final m?) {
      return ctx(m.group(1)!, [
        for (final (name, detail) in latexEnvironments)
          CompletionItem(name, detail, CompletionKind.environment, envBody: true, body: latexEnvBodies[name]),
      ]);
    }
    if (_cmd.firstMatch(before) case final m?) {
      final prefix = m.group(1)!;
      if (prefix.isEmpty && !explicit) return null;
      return ctx(prefix, [
        for (final (ins, caret, detail) in latexCommands)
          CompletionItem(ins.replaceAll(RegExp(r'[{\[].*$'), ''), detail, CompletionKind.command, insert: ins, caret: caret),
        for (final c in _defined(doc)) CompletionItem(c, 'defined in this file', CompletionKind.command),
        CompletionItem('begin', 'Environment', CompletionKind.environment, insert: 'begin{}', caret: 6),
      ]);
    }
    return null;
  }

  if (doc.lang == Lang.typst) {
    if (_typFile.firstMatch(before) case final m?) {
      return ctx(m.group(1)!, [
        for (final f in s.files)
          if (!f.startsWith('.')) CompletionItem(f, _ext(f), CompletionKind.file),
      ]);
    }
    if (_typRef.firstMatch(before) case final m?) {
      return ctx(m.group(1)!, [
        for (final (k, d) in s.labels) CompletionItem(k, d, CompletionKind.label),
        for (final (k, d) in s.citations) CompletionItem(k, d, CompletionKind.citation),
      ]);
    }
    if (_typFn.firstMatch(before) case final m?) {
      final prefix = m.group(1) ?? '';
      return ctx(prefix, [
        for (final (ins, caret, detail) in typstFunctions)
          CompletionItem(ins.replaceAll(RegExp(r'[(\[ ].*$'), ''), detail, CompletionKind.function, insert: ins, caret: caret),
      ]);
    }
  }
  return null;
}

class CompletionState extends ChangeNotifier {
  bool open = false;
  Pos? anchor;
  List<CompletionItem> items = const [];
  int index = 0;
  String prefix = '';

  CompletionItem? get selected => open && index < items.length ? items[index] : null;

  void show(Pos at, CompletionContext ctx) {
    final same = open && anchor == at;
    anchor = at;
    final keep = same ? selected?.label : null;
    items = ctx.items;
    prefix = ctx.prefix;
    index = keep == null ? 0 : math.max(0, items.indexWhere((i) => i.label == keep));
    open = true;
    notifyListeners();
  }

  void step(int d) {
    if (items.isEmpty) return;
    index = (index + d).clamp(0, items.length - 1);
    notifyListeners();
  }

  void close() {
    if (!open) return;
    open = false;
    notifyListeners();
  }
}

class CompletionPopup extends StatefulWidget {
  const CompletionPopup({super.key, required this.state, required this.onPick});
  final CompletionState state;
  final ValueChanged<int> onPick;
  @override
  State<CompletionPopup> createState() => _CompletionPopupState();
}

class _CompletionPopupState extends State<CompletionPopup> {
  final _scroll = ScrollController();
  static const _row = 28.0;

  @override
  void initState() {
    super.initState();
    widget.state.addListener(_changed);
  }

  @override
  void dispose() {
    widget.state.removeListener(_changed);
    _scroll.dispose();
    super.dispose();
  }

  void _changed() {
    if (!mounted) return;
    setState(() {});
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!_scroll.hasClients) return;
      final top = widget.state.index * _row;
      final view = _scroll.position.viewportDimension;
      if (top < _scroll.offset) _scroll.jumpTo(top);
      if (top + _row > _scroll.offset + view) _scroll.jumpTo(top + _row - view);
    });
  }

  IconData _icon(CompletionKind k) => switch (k) {
    CompletionKind.command => Icons.code,
    CompletionKind.environment => Icons.data_object,
    CompletionKind.label => Icons.label_outline,
    CompletionKind.citation => Icons.menu_book_outlined,
    CompletionKind.package => Icons.inventory_2_outlined,
    CompletionKind.file => Icons.insert_drive_file_outlined,
    CompletionKind.function => Icons.functions,
  };

  @override
  Widget build(BuildContext context) {
    final k = KT.of(context);
    final s = widget.state;
    final n = s.items.length;
    final mono = TextStyle(fontFamily: monoFamily, fontFamilyFallback: monoFallback, fontSize: 13, color: k.text);
    return Material(
      elevation: 8,
      color: k.surface,
      shadowColor: Colors.black38,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(8),
        side: BorderSide(color: k.border),
      ),
      clipBehavior: Clip.antiAlias,
      child: SizedBox(
        width: 420,
        height: math.min(n, 8) * _row + 4,
        child: ListView.builder(
          controller: _scroll,
          padding: const EdgeInsets.symmetric(vertical: 2),
          itemExtent: _row,
          itemCount: n,
          itemBuilder: (context, i) {
            final it = s.items[i];
            final on = i == s.index;
            final p = s.prefix.length;
            final hl = it.label.toLowerCase().startsWith(s.prefix.toLowerCase()) ? math.min(p, it.label.length) : 0;
            return InkWell(
              onTap: () => widget.onPick(i),
              child: Container(
                color: on ? k.accentSoft : null,
                padding: const EdgeInsets.symmetric(horizontal: 10),
                child: Row(
                  children: [
                    Icon(_icon(it.kind), size: 14, color: on ? k.accent : k.faint),
                    const SizedBox(width: 8),
                    Text.rich(
                      TextSpan(
                        style: mono,
                        children: [
                          TextSpan(
                            text: it.label.substring(0, hl),
                            style: TextStyle(color: k.accent, fontWeight: FontWeight.w700),
                          ),
                          TextSpan(text: it.label.substring(hl)),
                        ],
                      ),
                    ),
                    const SizedBox(width: 12),
                    Expanded(
                      child: Text(
                        it.detail,
                        textAlign: TextAlign.right,
                        overflow: TextOverflow.ellipsis,
                        style: TextStyle(color: k.muted, fontSize: 12),
                      ),
                    ),
                  ],
                ),
              ),
            );
          },
        ),
      ),
    );
  }
}
