import 'package:file_selector/file_selector.dart';
import 'package:material_ui/material_ui.dart';

import 'dialogs.dart';
import 'state.dart';
import 'theme.dart';

class StartScreen extends StatelessWidget {
  const StartScreen(this.m, {super.key});
  final AppModel m;

  String _ago(int t) {
    final s = (DateTime.now().millisecondsSinceEpoch - t) / 1000;
    if (s < 60) return 'just now';
    if (s < 3600) return '${s ~/ 60} min ago';
    if (s < 86400) return '${s ~/ 3600} h ago';
    final d = s ~/ 86400;
    return d == 1 ? 'yesterday' : '$d days ago';
  }

  Future<void> _openFolder() async {
    final d = await getDirectoryPath();
    if (d != null) await m.openProject(d);
  }

  Future<void> _importZip() async {
    final f = await openFile(
      acceptedTypeGroups: const [
        XTypeGroup(label: 'ZIP archive', extensions: ['zip']),
      ],
    );
    if (f != null) await m.importZip(f.path);
  }

  @override
  Widget build(BuildContext context) {
    final k = KT.of(context);
    Widget action(IconData icon, String title, String sub, VoidCallback onTap, [String? key]) => Material(
      color: k.surface,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(12),
        side: BorderSide(color: k.border),
      ),
      child: InkWell(
        borderRadius: BorderRadius.circular(12),
        hoverColor: k.hover,
        onTap: onTap,
        child: Padding(
          padding: const EdgeInsets.all(16),
          child: Row(
            children: [
              Icon(icon, color: k.accent, size: 22),
              const SizedBox(width: 14),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(title, style: const TextStyle(fontWeight: FontWeight.w600, fontSize: 14)),
                    const SizedBox(height: 2),
                    Text(sub, style: TextStyle(color: k.muted, fontSize: 12)),
                  ],
                ),
              ),
              if (key != null)
                Container(
                  padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
                  decoration: BoxDecoration(
                    color: k.surface2,
                    borderRadius: BorderRadius.circular(4),
                    border: Border.all(color: k.border),
                  ),
                  child: Text(key, style: TextStyle(color: k.muted, fontSize: 11)),
                ),
            ],
          ),
        ),
      ),
    );

    return SingleChildScrollView(
      padding: const EdgeInsets.symmetric(horizontal: 24, vertical: 48),
      child: Center(
        child: ConstrainedBox(
          constraints: const BoxConstraints(maxWidth: 760),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Row(
                children: [
                  Image.asset('assets/logo.png', width: 40, height: 40),
                  const SizedBox(width: 12),
                  const Text('KernTeX', style: TextStyle(fontSize: 30, fontWeight: FontWeight.w700, letterSpacing: -0.6)),
                ],
              ),
              const SizedBox(height: 6),
              Text(
                'Write LaTeX and Typst locally — with a live PDF, clear errors and nothing else to install.',
                style: TextStyle(color: k.muted, fontSize: 15),
              ),
              const SizedBox(height: 28),
              LayoutBuilder(
                builder: (context, box) {
                  final two = box.maxWidth > 560;
                  final items = [
                    action(
                      Icons.note_add_outlined,
                      'New project',
                      'Blank, article, thesis, slides, letter',
                      () => showNewProject(context, m),
                      'Ctrl+Shift+N',
                    ),
                    action(Icons.folder_open_outlined, 'Open folder', 'Any folder with .tex or .typ files', _openFolder, 'Ctrl+O'),
                    action(Icons.folder_zip_outlined, 'Import ZIP', 'e.g. Overleaf: Menu → Download → Source', _importZip),
                    action(Icons.auto_awesome_outlined, 'Example project', 'A guided tour of LaTeX', () => m.openWelcome('latex')),
                  ];
                  return Wrap(
                    spacing: 10,
                    runSpacing: 10,
                    children: [for (final i in items) SizedBox(width: two ? (box.maxWidth - 10) / 2 : box.maxWidth, child: i)],
                  );
                },
              ),
              const SizedBox(height: 14),
              Text.rich(
                TextSpan(
                  style: TextStyle(color: k.muted, fontSize: 13),
                  children: [
                    const TextSpan(text: 'New to LaTeX or curious about something simpler? '),
                    WidgetSpan(
                      alignment: PlaceholderAlignment.baseline,
                      baseline: TextBaseline.alphabetic,
                      child: InkWell(
                        onTap: () => m.openWelcome('typst'),
                        child: Text(
                          'Take the Typst tour',
                          style: TextStyle(color: k.accent, fontWeight: FontWeight.w600, fontSize: 13),
                        ),
                      ),
                    ),
                    const TextSpan(text: ' — a modern alternative that compiles instantly.'),
                  ],
                ),
              ),
              const SizedBox(height: 34),
              Text(
                'RECENT PROJECTS',
                style: TextStyle(color: k.muted, fontSize: 12, letterSpacing: 0.7, fontWeight: FontWeight.w600),
              ),
              const SizedBox(height: 8),
              if (m.recents.isEmpty)
                Container(
                  width: double.infinity,
                  padding: const EdgeInsets.all(18),
                  decoration: BoxDecoration(
                    borderRadius: BorderRadius.circular(12),
                    border: Border.all(color: k.borderStrong),
                  ),
                  child: Text('No projects yet. Create one or open a folder.', style: TextStyle(color: k.muted)),
                )
              else
                Container(
                  clipBehavior: Clip.antiAlias,
                  decoration: BoxDecoration(
                    color: k.surface,
                    borderRadius: BorderRadius.circular(12),
                    border: Border.all(color: k.border),
                  ),
                  child: Column(
                    children: [
                      for (final (i, r) in m.recents.indexed) ...[
                        if (i > 0) Divider(height: 1, color: k.border),
                        _RecentRow(r: r, ago: _ago(r.openedAt), m: m),
                      ],
                    ],
                  ),
                ),
              const SizedBox(height: 24),
              Text(
                'Tip: press Ctrl+Enter to compile, Ctrl+S to save, Ctrl+B to toggle the file tree.',
                style: TextStyle(color: k.muted, fontSize: 12),
              ),
              const SizedBox(height: 28),
              Row(
                children: [
                  Text('KernTeX $appVersion · Flutter preview', style: TextStyle(color: k.muted, fontSize: 12)),
                  const SizedBox(width: 10),
                  InkWell(
                    onTap: () => showAbout(context),
                    child: Text(
                      'About',
                      style: TextStyle(color: k.muted, fontSize: 12, decoration: TextDecoration.underline),
                    ),
                  ),
                  const SizedBox(width: 10),
                  InkWell(
                    onTap: () => showSettings(context, m),
                    child: Text(
                      'Settings',
                      style: TextStyle(color: k.muted, fontSize: 12, decoration: TextDecoration.underline),
                    ),
                  ),
                ],
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _RecentRow extends StatefulWidget {
  const _RecentRow({required this.r, required this.ago, required this.m});
  final Recent r;
  final String ago;
  final AppModel m;
  @override
  State<_RecentRow> createState() => _RecentRowState();
}

class _RecentRowState extends State<_RecentRow> {
  bool hover = false;
  @override
  Widget build(BuildContext context) {
    final k = KT.of(context);
    return MouseRegion(
      onEnter: (_) => setState(() => hover = true),
      onExit: (_) => setState(() => hover = false),
      child: InkWell(
        onTap: () => widget.m.openProject(widget.r.root),
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
          child: Row(
            children: [
              Icon(Icons.folder_outlined, size: 16, color: k.muted),
              const SizedBox(width: 10),
              Text(widget.r.name, style: const TextStyle(fontWeight: FontWeight.w600)),
              const SizedBox(width: 10),
              Expanded(
                child: Text(
                  widget.r.root,
                  style: TextStyle(color: k.muted, fontSize: 12),
                  overflow: TextOverflow.ellipsis,
                ),
              ),
              Text(widget.ago, style: TextStyle(color: k.muted, fontSize: 12)),
              const SizedBox(width: 6),
              Opacity(
                opacity: hover ? 1 : 0,
                child: IconButton(
                  tooltip: 'Remove from list (the folder is not touched)',
                  icon: const Icon(Icons.close, size: 14),
                  onPressed: () => widget.m.removeRecent(widget.r.root),
                  constraints: const BoxConstraints(minWidth: 26, minHeight: 26),
                  padding: EdgeInsets.zero,
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
