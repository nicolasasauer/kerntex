import 'dart:io';

import 'package:file_selector/file_selector.dart';
import 'package:flutter/services.dart';
import 'package:material_ui/material_ui.dart';

import 'dialogs.dart';
import 'editor/editor_view.dart';
import 'native.dart';
import 'pdf_view.dart';
import 'state.dart';
import 'theme.dart';

class Workspace extends StatefulWidget {
  const Workspace(this.m, {super.key});
  final AppModel m;
  @override
  State<Workspace> createState() => WorkspaceState();
}

class WorkspaceState extends State<Workspace> {
  final _editors = <String, GlobalKey<EditorViewState>>{};
  final _focus = <String, FocusNode>{};
  AppModel get m => widget.m;

  @override
  void dispose() {
    for (final f in _focus.values) {
      f.dispose();
    }
    super.dispose();
  }

  Future<void> jumpTo(Map<String, dynamic> d) async {
    final file = d['file'] as String?;
    if (file == null) return;
    await m.openFile(file);
    final line = d['line'] as int?;
    if (line == null) return;
    WidgetsBinding.instance.addPostFrameCallback((_) => _editors[file]?.currentState?.reveal(line, d['column'] as int?));
  }

  @override
  Widget build(BuildContext context) {
    final k = KT.of(context);
    return Column(
      children: [
        _Toolbar(m),
        Expanded(
          child: LayoutBuilder(
            builder: (context, box) {
              final treeW = m.treeOpen ? m.treeWidth.clamp(160.0, box.maxWidth * 0.4) : 0.0;
              final rest = box.maxWidth - treeW;
              final pdfW = m.pdfOpen ? (rest * m.pdfFraction).clamp(240.0, rest - 280) : 0.0;
              return Row(
                children: [
                  if (m.treeOpen) ...[
                    SizedBox(width: treeW - 1, child: _FileTree(m)),
                    _Splitter(axis: Axis.horizontal, onDrag: (dx) => m.update(() => m.treeWidth = (m.treeWidth + dx).clamp(160, 480))),
                  ],
                  Expanded(
                    child: Column(
                      children: [
                        Expanded(child: _editorArea(k)),
                        if (m.problemsOpen) ...[
                          _Splitter(
                            axis: Axis.vertical,
                            onDrag: (dy) => m.update(() => m.problemsHeight = (m.problemsHeight - dy).clamp(90, box.maxHeight - 120)),
                          ),
                          SizedBox(
                            height: m.problemsHeight,
                            child: _BottomPanel(m, onJump: jumpTo),
                          ),
                        ],
                      ],
                    ),
                  ),
                  if (m.pdfOpen) ...[
                    _Splitter(axis: Axis.horizontal, onDrag: (dx) => m.update(() => m.pdfFraction = ((pdfW - dx) / rest).clamp(0.2, 0.75))),
                    SizedBox(
                      width: pdfW - 1,
                      child: m.pdfPath == null
                          ? ColoredBox(
                              color: k.pdfBg,
                              child: Center(
                                child: Text('The PDF appears here after the first build.', style: TextStyle(color: k.muted)),
                              ),
                            )
                          : PdfView(path: m.pdfPath!, version: m.pdfVersion),
                    ),
                  ],
                ],
              );
            },
          ),
        ),
        _StatusBar(m),
      ],
    );
  }

  Widget _editorArea(KT k) {
    final preview = m.previewPath;
    return Column(
      children: [
        _Tabs(m),
        Expanded(
          child: preview != null
              ? _Preview(m, preview)
              : m.active == null
              ? ColoredBox(
                  color: k.editorBg,
                  child: Center(
                    child: Text('Open a file from the tree on the left.', style: TextStyle(color: k.muted)),
                  ),
                )
              : Builder(
                  builder: (context) {
                    final d = m.active!;
                    final key = _editors.putIfAbsent(d.path, () => GlobalKey<EditorViewState>());
                    final focus = _focus.putIfAbsent(d.path, () => FocusNode());
                    return EditorView(key: key, doc: d.doc, fontSize: m.fontSize, focusNode: focus, source: m.completions);
                  },
                ),
        ),
      ],
    );
  }
}

class _Splitter extends StatelessWidget {
  const _Splitter({required this.axis, required this.onDrag});
  final Axis axis;
  final ValueChanged<double> onDrag;
  @override
  Widget build(BuildContext context) {
    final k = KT.of(context);
    final h = axis == Axis.horizontal;
    return MouseRegion(
      cursor: h ? SystemMouseCursors.resizeColumn : SystemMouseCursors.resizeRow,
      child: GestureDetector(
        behavior: HitTestBehavior.opaque,
        onHorizontalDragUpdate: h ? (d) => onDrag(d.delta.dx) : null,
        onVerticalDragUpdate: h ? null : (d) => onDrag(d.delta.dy),
        child: Container(width: h ? 1 : null, height: h ? null : 1, color: k.border),
      ),
    );
  }
}

Widget _iconBtn(BuildContext context, IconData icon, String tip, VoidCallback? onTap, {bool active = false}) {
  final k = KT.of(context);
  return IconButton(
    icon: Icon(icon, size: 18, color: active ? k.accent : k.muted),
    tooltip: tip,
    onPressed: onTap,
    constraints: const BoxConstraints(minWidth: 32, minHeight: 32),
    padding: EdgeInsets.zero,
    style: IconButton.styleFrom(shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(6))),
  );
}

class _Toolbar extends StatelessWidget {
  const _Toolbar(this.m);
  final AppModel m;
  @override
  Widget build(BuildContext context) {
    final k = KT.of(context);
    final running = m.build == BuildState.running;
    return Container(
      height: 46,
      padding: const EdgeInsets.symmetric(horizontal: 8),
      decoration: BoxDecoration(
        color: k.surface,
        border: Border(bottom: BorderSide(color: k.border)),
      ),
      child: Row(
        children: [
          _iconBtn(
            context,
            Icons.vertical_split_outlined,
            'Toggle file tree (Ctrl+B)',
            () => m.update(() => m.treeOpen = !m.treeOpen),
            active: m.treeOpen,
          ),
          const SizedBox(width: 6),
          InkWell(
            onTap: () => m.closeProject(),
            borderRadius: BorderRadius.circular(6),
            child: Padding(padding: const EdgeInsets.all(4), child: Image.asset('assets/logo.png', width: 22, height: 22)),
          ),
          const SizedBox(width: 8),
          Text(m.name ?? '', style: const TextStyle(fontWeight: FontWeight.w600)),
          if (m.mainFile != null) ...[Text('  /  ', style: TextStyle(color: k.faint)), Text(m.mainFile!, style: TextStyle(color: k.muted))],
          const Spacer(),
          Container(
            padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
            decoration: BoxDecoration(
              color: k.surface2,
              borderRadius: BorderRadius.circular(5),
              border: Border.all(color: k.border),
            ),
            child: Text(switch (m.engine) {
              'typst' => 'Typst',
              'latexmk' => 'latexmk · ${m.flavor}',
              _ => 'Tectonic',
            }, style: TextStyle(color: k.muted, fontSize: 12)),
          ),
          const SizedBox(width: 8),
          SizedBox(
            height: 32,
            child: running
                ? OutlinedButton.icon(
                    onPressed: m.cancelCompile,
                    icon: SizedBox(width: 14, height: 14, child: CircularProgressIndicator(strokeWidth: 2, color: k.accent)),
                    label: const Text('Stop'),
                    style: OutlinedButton.styleFrom(padding: const EdgeInsets.symmetric(horizontal: 12)),
                  )
                : FilledButton.icon(
                    onPressed: m.compile,
                    icon: const Icon(Icons.play_arrow_rounded, size: 18),
                    label: const Text('Compile'),
                    style: FilledButton.styleFrom(padding: const EdgeInsets.symmetric(horizontal: 12)),
                  ),
          ),
          const SizedBox(width: 6),
          _iconBtn(context, Icons.download_outlined, 'Export PDF…', m.pdfPath == null ? null : () => _exportPdf(m)),
          _iconBtn(
            context,
            Icons.picture_as_pdf_outlined,
            'Toggle PDF preview (Ctrl+Shift+P)',
            () => m.update(() => m.pdfOpen = !m.pdfOpen),
            active: m.pdfOpen,
          ),
          _iconBtn(context, Icons.settings_outlined, 'Settings (Ctrl+,)', () => showSettings(context, m)),
        ],
      ),
    );
  }
}

Future<void> _exportPdf(AppModel m) async {
  final stem = (m.mainFile ?? 'document').split('/').last.replaceAll(RegExp(r'\.(tex|typ)$'), '');
  final loc = await getSaveLocation(
    suggestedName: '$stem.pdf',
    acceptedTypeGroups: const [
      XTypeGroup(label: 'PDF', extensions: ['pdf']),
    ],
  );
  if (loc == null) return;
  try {
    Core.now('exportPdf', {'root': m.root, 'mainFile': m.mainFile, 'dest': loc.path});
    m.showToast('PDF exported');
  } catch (e) {
    m.fail(e);
  }
}

class _Tabs extends StatelessWidget {
  const _Tabs(this.m);
  final AppModel m;
  @override
  Widget build(BuildContext context) {
    final k = KT.of(context);
    return Container(
      height: 36,
      decoration: BoxDecoration(
        color: k.surface2,
        border: Border(bottom: BorderSide(color: k.border)),
      ),
      child: ListView(
        scrollDirection: Axis.horizontal,
        children: [
          for (final d in m.docs)
            ListenableBuilder(
              listenable: d.doc,
              builder: (context, _) {
                final on = d.path == m.activePath && m.previewPath == null;
                return InkWell(
                  onTap: () => m.openFile(d.path),
                  child: Container(
                    padding: const EdgeInsets.only(left: 12, right: 4),
                    decoration: BoxDecoration(
                      color: on ? k.editorBg : null,
                      border: Border(
                        right: BorderSide(color: k.border),
                        top: BorderSide(color: on ? k.accent : Colors.transparent, width: 2),
                      ),
                    ),
                    child: Row(
                      children: [
                        Text(d.name, style: TextStyle(color: on ? k.text : k.muted, fontSize: 13)),
                        if (d.path == m.mainFile)
                          Padding(
                            padding: const EdgeInsets.only(left: 6),
                            child: Icon(Icons.star_rounded, size: 12, color: k.accent),
                          ),
                        SizedBox(
                          width: 26,
                          child: d.dirty
                              ? Center(
                                  child: Container(
                                    width: 7,
                                    height: 7,
                                    decoration: BoxDecoration(color: k.muted, shape: BoxShape.circle),
                                  ),
                                )
                              : IconButton(
                                  icon: const Icon(Icons.close, size: 13),
                                  onPressed: () => m.closeDoc(d),
                                  padding: EdgeInsets.zero,
                                  constraints: const BoxConstraints(minWidth: 22, minHeight: 22),
                                ),
                        ),
                      ],
                    ),
                  ),
                );
              },
            ),
        ],
      ),
    );
  }
}

class _Preview extends StatelessWidget {
  const _Preview(this.m, this.path);
  final AppModel m;
  final String path;
  @override
  Widget build(BuildContext context) {
    final k = KT.of(context);
    final abs = '${m.root}${Platform.pathSeparator}${path.replaceAll('/', Platform.pathSeparator)}';
    if (ext(path) == 'pdf') return PdfView(path: abs, version: 0, compact: true);
    return ColoredBox(
      color: k.pdfBg,
      child: Center(
        child: Padding(
          padding: const EdgeInsets.all(24),
          child: Image.file(File(abs), filterQuality: FilterQuality.medium, errorBuilder: (_, e, _) => Text('Can\'t show this image: $e')),
        ),
      ),
    );
  }
}

// ── File tree ─────────────────────────────────────────────────────────────

class _FileTree extends StatefulWidget {
  const _FileTree(this.m);
  final AppModel m;
  @override
  State<_FileTree> createState() => _FileTreeState();
}

class _FileTreeState extends State<_FileTree> {
  final _collapsed = <String>{};
  AppModel get m => widget.m;

  List<(Node, int)> _flatten(List<Node> nodes, int depth, [List<(Node, int)>? out]) {
    out ??= [];
    for (final n in nodes) {
      out.add((n, depth));
      if (n.isDir && !_collapsed.contains(n.path)) _flatten(n.children, depth + 1, out);
    }
    return out;
  }

  Future<void> _menu(BuildContext context, Offset pos, Node? n) async {
    final dir = n == null ? '' : (n.isDir ? n.path : (n.path.contains('/') ? n.path.substring(0, n.path.lastIndexOf('/')) : ''));
    final isMainCandidate = n != null && !n.isDir && (n.path.endsWith('.tex') || n.path.endsWith('.typ')) && n.path != m.mainFile;
    final choice = await showMenu<String>(
      context: context,
      position: RelativeRect.fromLTRB(pos.dx, pos.dy, pos.dx, pos.dy),
      items: [
        const PopupMenuItem(value: 'newFile', height: 34, child: Text('New file…')),
        const PopupMenuItem(value: 'newFolder', height: 34, child: Text('New folder…')),
        const PopupMenuItem(value: 'add', height: 34, child: Text('Add files…')),
        if (isMainCandidate) const PopupMenuItem(value: 'main', height: 34, child: Text('Set as main file')),
        if (n != null) ...[
          const PopupMenuDivider(),
          const PopupMenuItem(value: 'rename', height: 34, child: Text('Rename…')),
          const PopupMenuItem(value: 'reveal', height: 34, child: Text('Show in file manager')),
          const PopupMenuItem(value: 'trash', height: 34, child: Text('Move to trash')),
        ],
      ],
    );
    if (!context.mounted) return;
    switch (choice) {
      case 'newFile' || 'newFolder':
        final name = await prompt(
          context,
          choice == 'newFile' ? 'New file' : 'New folder',
          initial: choice == 'newFile' ? (m.isTypst ? 'chapter.typ' : 'chapter.tex') : 'images',
          action: 'Create',
        );
        if (name != null && name.isNotEmpty) await m.createEntry(dir, name, folder: choice == 'newFolder');
      case 'add':
        final files = await openFiles();
        if (files.isNotEmpty) await m.importFiles(dir, [for (final f in files) f.path]);
      case 'main':
        await m.setMain(n!.path);
      case 'rename':
        final name = await prompt(context, 'Rename', initial: n!.name, action: 'Rename');
        if (name != null && name.isNotEmpty && name != n.name) {
          final parent = n.path.contains('/') ? n.path.substring(0, n.path.lastIndexOf('/') + 1) : '';
          await m.rename(n.path, '$parent$name');
        }
      case 'reveal':
        Core.call('reveal', {'path': '${m.root}/${n!.path}'}).catchError((e) => m.fail(e));
      case 'trash':
        await m.trash(n!.path);
    }
  }

  @override
  Widget build(BuildContext context) {
    final k = KT.of(context);
    final rows = _flatten(m.tree, 0);
    return Container(
      color: k.surface,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Padding(
            padding: const EdgeInsets.fromLTRB(14, 10, 6, 4),
            child: Row(
              children: [
                Expanded(
                  child: Text(
                    'FILES',
                    style: TextStyle(color: k.muted, fontSize: 11, letterSpacing: 0.7, fontWeight: FontWeight.w600),
                  ),
                ),
                Builder(
                  builder: (c) => _iconBtn(c, Icons.add, 'New file, folder or upload', () {
                    final box = c.findRenderObject() as RenderBox;
                    _menu(c, box.localToGlobal(Offset(0, box.size.height)), null);
                  }),
                ),
              ],
            ),
          ),
          Expanded(
            child: GestureDetector(
              onSecondaryTapUp: (d) => _menu(context, d.globalPosition, null),
              child: ListView.builder(
                itemCount: rows.length,
                itemExtent: 28,
                itemBuilder: (context, i) {
                  final (n, depth) = rows[i];
                  final on = n.path == m.activePath || n.path == m.previewPath;
                  final icon = n.isDir
                      ? (_collapsed.contains(n.path) ? Icons.folder_outlined : Icons.folder_open_outlined)
                      : switch (ext(n.path)) {
                          'tex' || 'typ' => Icons.description_outlined,
                          'bib' => Icons.menu_book_outlined,
                          'pdf' => Icons.picture_as_pdf_outlined,
                          'png' || 'jpg' || 'jpeg' || 'gif' || 'webp' => Icons.image_outlined,
                          _ => Icons.insert_drive_file_outlined,
                        };
                  return GestureDetector(
                    onSecondaryTapUp: (d) => _menu(context, d.globalPosition, n),
                    child: InkWell(
                      hoverColor: k.hover,
                      onTap: () {
                        if (n.isDir) {
                          setState(() => _collapsed.contains(n.path) ? _collapsed.remove(n.path) : _collapsed.add(n.path));
                        } else {
                          m.openFile(n.path);
                        }
                      },
                      child: Container(
                        color: on ? k.accentSoft : null,
                        padding: EdgeInsets.only(left: 10.0 + depth * 14, right: 8),
                        child: Row(
                          children: [
                            Icon(icon, size: 16, color: n.generated ? k.faint : (on ? k.accent : k.muted)),
                            const SizedBox(width: 7),
                            Expanded(
                              child: Text(
                                n.name,
                                overflow: TextOverflow.ellipsis,
                                style: TextStyle(
                                  fontSize: 13,
                                  color: n.generated ? k.faint : k.text,
                                  fontWeight: n.path == m.mainFile ? FontWeight.w600 : null,
                                ),
                              ),
                            ),
                            if (n.path == m.mainFile)
                              Tooltip(
                                message: 'Main file',
                                child: Icon(Icons.star_rounded, size: 13, color: k.accent),
                              ),
                          ],
                        ),
                      ),
                    ),
                  );
                },
              ),
            ),
          ),
        ],
      ),
    );
  }
}

// ── Bottom panel: problems, build details, raw log ───────────────────────

class _BottomPanel extends StatelessWidget {
  const _BottomPanel(this.m, {required this.onJump});
  final AppModel m;
  final ValueChanged<Map<String, dynamic>> onJump;

  @override
  Widget build(BuildContext context) {
    final k = KT.of(context);
    Widget tab(String id, String label) {
      final on = m.bottomTab == id;
      return InkWell(
        onTap: () => m.update(() => m.bottomTab = id),
        child: Container(
          padding: const EdgeInsets.symmetric(horizontal: 12),
          alignment: Alignment.center,
          decoration: BoxDecoration(
            border: Border(bottom: BorderSide(color: on ? k.accent : Colors.transparent, width: 2)),
          ),
          child: Text(
            label,
            style: TextStyle(fontSize: 12.5, color: on ? k.text : k.muted, fontWeight: on ? FontWeight.w600 : null),
          ),
        ),
      );
    }

    final errors = m.count('error'), warnings = m.count('warning');
    return Container(
      color: k.surface,
      child: Column(
        children: [
          Container(
            height: 34,
            decoration: BoxDecoration(
              border: Border(bottom: BorderSide(color: k.border)),
            ),
            child: Row(
              children: [
                tab('problems', 'Problems${m.diagnostics.isEmpty ? '' : ' (${m.diagnostics.length})'}'),
                tab('details', 'Build details'),
                tab('log', 'Raw log'),
                const Spacer(),
                if (errors + warnings > 0)
                  Text('${_n(errors, 'error')} · ${_n(warnings, 'warning')}', style: TextStyle(color: k.muted, fontSize: 12)),
                _iconBtn(context, Icons.close, 'Close panel (Ctrl+J)', () => m.update(() => m.problemsOpen = false)),
              ],
            ),
          ),
          Expanded(
            child: switch (m.bottomTab) {
              'log' => _Log(m.result?['log'] as String? ?? ''),
              'details' => _Details(m),
              _ => _Problems(m, onJump),
            },
          ),
        ],
      ),
    );
  }
}

class _Problems extends StatelessWidget {
  const _Problems(this.m, this.onJump);
  final AppModel m;
  final ValueChanged<Map<String, dynamic>> onJump;

  @override
  Widget build(BuildContext context) {
    final k = KT.of(context);
    if (m.diagnostics.isEmpty) {
      return Center(
        child: Text(m.result == null ? 'Compile to check your document.' : 'No problems — nice.', style: TextStyle(color: k.muted)),
      );
    }
    return ListView.separated(
      padding: const EdgeInsets.symmetric(vertical: 4),
      itemCount: m.diagnostics.length,
      separatorBuilder: (_, _) => Divider(height: 1, color: k.border),
      itemBuilder: (context, i) {
        final d = m.diagnostics[i];
        final sev = d['severity'] as String? ?? 'error';
        final color = switch (sev) {
          'warning' => k.warning,
          'info' => k.info,
          _ => k.danger,
        };
        final where = [d['file'], d['line']].whereType<Object>().join(':');
        return InkWell(
          onTap: d['file'] == null ? null : () => onJump(d),
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 8),
            child: Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Icon(
                  sev == 'error' ? Icons.error_outline : (sev == 'warning' ? Icons.warning_amber_rounded : Icons.info_outline),
                  size: 17,
                  color: color,
                ),
                const SizedBox(width: 10),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      SelectableText(d['message'] as String? ?? '', style: const TextStyle(fontWeight: FontWeight.w500)),
                      if (d['hint'] != null)
                        Padding(
                          padding: const EdgeInsets.only(top: 3),
                          child: Text(d['hint'] as String, style: TextStyle(color: k.muted, fontSize: 12.5)),
                        ),
                    ],
                  ),
                ),
                const SizedBox(width: 12),
                Text(
                  where,
                  style: TextStyle(color: k.accent, fontSize: 12, fontFamily: monoFamily, fontFamilyFallback: monoFallback),
                ),
                const SizedBox(width: 8),
                Text(d['source'] as String? ?? '', style: TextStyle(color: k.faint, fontSize: 11)),
              ],
            ),
          ),
        );
      },
    );
  }
}

class _Log extends StatelessWidget {
  const _Log(this.log);
  final String log;
  @override
  Widget build(BuildContext context) {
    final k = KT.of(context);
    if (log.isEmpty) {
      return Center(
        child: Text('No log yet.', style: TextStyle(color: k.muted)),
      );
    }
    final lines = log.split('\n');
    // Virtualised: a 10 000-line TeX log costs no more than one screen.
    return SelectionArea(
      child: ListView.builder(
        padding: const EdgeInsets.all(12),
        itemCount: lines.length,
        itemExtent: 18,
        itemBuilder: (_, i) => Text(
          lines[i],
          softWrap: false,
          overflow: TextOverflow.clip,
          style: TextStyle(fontFamily: monoFamily, fontFamilyFallback: monoFallback, fontSize: 12, color: k.muted),
        ),
      ),
    );
  }
}

class _Details extends StatelessWidget {
  const _Details(this.m);
  final AppModel m;
  @override
  Widget build(BuildContext context) {
    final k = KT.of(context);
    final r = m.result;
    final d = r?['details'] as Map<String, dynamic>?;
    if (r == null || d == null) {
      return Center(
        child: Text('Compile once to see which engine and packages your document uses.', style: TextStyle(color: k.muted)),
      );
    }
    final pkgs = [for (final p in d['packages'] as List? ?? const []) p as Map<String, dynamic>];
    Widget fact(String key, String? value) => value == null
        ? const SizedBox.shrink()
        : Padding(
            padding: const EdgeInsets.symmetric(vertical: 2),
            child: Row(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                SizedBox(
                  width: 130,
                  child: Text(key, style: TextStyle(color: k.muted)),
                ),
                Expanded(child: SelectableText(value)),
              ],
            ),
          );
    final cls = d['documentClass'] as Map<String, dynamic>?;
    return ListView(
      padding: const EdgeInsets.fromLTRB(16, 10, 16, 16),
      children: [
        fact('Engine', r['engineLabel'] as String?),
        fact('TeX program', d['program'] as String?),
        fact('LaTeX format', d['format'] as String?),
        fact('Document class', cls == null ? null : [cls['name'], cls['version'], cls['date']].whereType<String>().join(' · ')),
        fact('Last build', '${d['passes'] ?? 1} pass(es) · ${(((r['durationMs'] as num?) ?? 0) / 1000).toStringAsFixed(1)} s'),
        fact('Output folder', d['outDir'] as String?),
        const SizedBox(height: 12),
        Text('Packages (${pkgs.length})', style: const TextStyle(fontWeight: FontWeight.w600)),
        const SizedBox(height: 6),
        if (pkgs.isEmpty) Text('The document doesn\'t load any packages.', style: TextStyle(color: k.muted)),
        for (final p in pkgs)
          Padding(
            padding: const EdgeInsets.symmetric(vertical: 2),
            child: Row(
              children: [
                SizedBox(
                  width: 190,
                  child: Text(
                    p['name'] as String,
                    style: TextStyle(fontFamily: monoFamily, fontFamilyFallback: monoFallback, fontSize: 12),
                  ),
                ),
                SizedBox(
                  width: 150,
                  child: Text([p['version'], p['date']].whereType<String>().join(' · '), style: TextStyle(color: k.muted, fontSize: 12)),
                ),
                Expanded(
                  child: Text(
                    p['description'] as String? ?? '',
                    style: TextStyle(color: k.muted, fontSize: 12),
                    overflow: TextOverflow.ellipsis,
                  ),
                ),
              ],
            ),
          ),
      ],
    );
  }
}

class _StatusBar extends StatelessWidget {
  const _StatusBar(this.m);
  final AppModel m;
  @override
  Widget build(BuildContext context) {
    final k = KT.of(context);
    final r = m.result;
    final errors = m.count('error'), warnings = m.count('warning');
    final String text;
    final Color color;
    if (m.build == BuildState.running) {
      text = m.progress ?? 'Compiling…';
      color = k.muted;
    } else if (r == null && m.diagnostics.isEmpty) {
      text = 'Ready';
      color = k.muted;
    } else if (errors > 0) {
      text = errors == 1 ? '1 error — see Problems' : '$errors errors — see Problems';
      color = k.danger;
    } else {
      text =
          'Compiled in ${(((r?['durationMs'] as num?) ?? 0) / 1000).toStringAsFixed(1)} s${warnings > 0 ? ' · ${_n(warnings, 'warning')}' : ''}';
      color = k.success;
    }
    final a = m.active;
    return Container(
      height: 26,
      padding: const EdgeInsets.symmetric(horizontal: 12),
      decoration: BoxDecoration(
        color: k.surface,
        border: Border(top: BorderSide(color: k.border)),
      ),
      child: Row(
        children: [
          InkWell(
            onTap: () => m.update(() => m.problemsOpen = !m.problemsOpen),
            child: Row(
              children: [
                Icon(
                  m.build == BuildState.running
                      ? Icons.hourglass_top_rounded
                      : (errors > 0 ? Icons.error_outline : Icons.check_circle_outline),
                  size: 14,
                  color: color,
                ),
                const SizedBox(width: 6),
                Text(text, style: TextStyle(color: color, fontSize: 12)),
              ],
            ),
          ),
          const Spacer(),
          if (m.toast != null) Text(m.toast!, style: TextStyle(color: k.text, fontSize: 12)),
          const SizedBox(width: 16),
          if (a != null)
            ListenableBuilder(
              listenable: a.doc,
              builder: (context, _) {
                final sel = a.doc.sel;
                final n = sel.isEmpty ? '' : ' (${a.doc.selectedText.length} selected)';
                return Text('Ln ${sel.head.line + 1}, Col ${sel.head.col + 1}$n', style: TextStyle(color: k.muted, fontSize: 12));
              },
            ),
        ],
      ),
    );
  }
}

/// Global keyboard shortcuts, active anywhere in the window.
Map<ShortcutActivator, VoidCallback> shortcuts(BuildContext context, AppModel m) => {
  const SingleActivator(LogicalKeyboardKey.keyS, control: true): () => m.root != null ? m.save() : null,
  const SingleActivator(LogicalKeyboardKey.enter, control: true): () => m.root != null ? m.compile() : null,
  const SingleActivator(LogicalKeyboardKey.keyB, control: true): () => m.root != null ? m.update(() => m.treeOpen = !m.treeOpen) : null,
  const SingleActivator(LogicalKeyboardKey.keyJ, control: true): () =>
      m.root != null ? m.update(() => m.problemsOpen = !m.problemsOpen) : null,
  const SingleActivator(LogicalKeyboardKey.keyP, control: true, shift: true): () =>
      m.root != null ? m.update(() => m.pdfOpen = !m.pdfOpen) : null,
  const SingleActivator(LogicalKeyboardKey.comma, control: true): () => showSettings(context, m),
  const SingleActivator(LogicalKeyboardKey.keyN, control: true, shift: true): () => showNewProject(context, m),
  const SingleActivator(LogicalKeyboardKey.keyO, control: true): () async {
    final d = await getDirectoryPath();
    if (d != null) await m.openProject(d);
  },
  const SingleActivator(LogicalKeyboardKey.equal, control: true): () => m.update(() => m.fontSize = (m.fontSize + 1).clamp(9, 28)),
  const SingleActivator(LogicalKeyboardKey.minus, control: true): () => m.update(() => m.fontSize = (m.fontSize - 1).clamp(9, 28)),
  const SingleActivator(LogicalKeyboardKey.escape): () => m.build == BuildState.running ? m.cancelCompile() : null,
};

String _n(int n, String word) => '$n $word${n == 1 ? '' : 's'}';
