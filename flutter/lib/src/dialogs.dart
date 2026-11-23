import 'package:file_selector/file_selector.dart';
import 'package:material_ui/material_ui.dart';

import 'native.dart';
import 'state.dart';
import 'theme.dart';

const appVersion = '0.1.3';
const repoUrl = 'https://github.com/nicolasasauer/kerntex';

const _templates = {
  'latex': [
    ('article', 'Article', 'Paper or short report with abstract'),
    ('report', 'Thesis / report', 'Chapters, table of contents'),
    ('beamer', 'Presentation', 'Beamer slides'),
    ('letter', 'Letter', 'Formal letter'),
    ('blank', 'Blank', 'Just the essentials'),
  ],
  'typst': [
    ('typstArticle', 'Article', 'Paper or short report with abstract'),
    ('typstReport', 'Thesis / report', 'Chapters, table of contents'),
    ('typstSlides', 'Presentation', '16:9 slides'),
    ('typstLetter', 'Letter', 'Formal letter'),
    ('typstBlank', 'Blank', 'Just the essentials'),
  ],
};

Future<void> showNewProject(BuildContext context, AppModel m) => showDialog(context: context, builder: (_) => _NewProject(m));

class _NewProject extends StatefulWidget {
  const _NewProject(this.m);
  final AppModel m;
  @override
  State<_NewProject> createState() => _NewProjectState();
}

class _NewProjectState extends State<_NewProject> {
  String format = 'latex';
  String template = 'article';
  final name = TextEditingController(text: 'My document');
  late String parent = (() {
    try {
      return Core.now('documentsDir') as String;
    } catch (_) {
      return '';
    }
  })();

  @override
  Widget build(BuildContext context) {
    final k = KT.of(context);
    return AlertDialog(
      title: const Text('New project', style: TextStyle(fontSize: 18, fontWeight: FontWeight.w600)),
      content: SizedBox(
        width: 520,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            SegmentedButton<String>(
              segments: const [
                ButtonSegment(value: 'latex', label: Text('LaTeX')),
                ButtonSegment(value: 'typst', label: Text('Typst')),
              ],
              selected: {format},
              showSelectedIcon: false,
              onSelectionChanged: (s) => setState(() {
                format = s.first;
                template = _templates[format]!.first.$1;
              }),
            ),
            const SizedBox(height: 14),
            Wrap(
              spacing: 8,
              runSpacing: 8,
              children: [
                for (final (id, title, desc) in _templates[format]!)
                  InkWell(
                    borderRadius: BorderRadius.circular(8),
                    onTap: () => setState(() => template = id),
                    child: Container(
                      width: 164,
                      padding: const EdgeInsets.all(10),
                      decoration: BoxDecoration(
                        color: template == id ? k.accentSoft : k.surface,
                        borderRadius: BorderRadius.circular(8),
                        border: Border.all(color: template == id ? k.accent : k.border),
                      ),
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text(title, style: const TextStyle(fontWeight: FontWeight.w600)),
                          const SizedBox(height: 2),
                          Text(desc, style: TextStyle(color: k.muted, fontSize: 12)),
                        ],
                      ),
                    ),
                  ),
              ],
            ),
            const SizedBox(height: 16),
            TextField(
              controller: name,
              autofocus: true,
              decoration: const InputDecoration(labelText: 'Name'),
            ),
            const SizedBox(height: 10),
            Row(
              children: [
                Expanded(
                  child: Text(
                    'Location: $parent',
                    style: TextStyle(color: k.muted, fontSize: 12),
                    overflow: TextOverflow.ellipsis,
                  ),
                ),
                TextButton(
                  onPressed: () async {
                    final d = await getDirectoryPath(initialDirectory: parent);
                    if (d != null) setState(() => parent = d);
                  },
                  child: const Text('Change…'),
                ),
              ],
            ),
          ],
        ),
      ),
      actions: [
        TextButton(onPressed: () => Navigator.pop(context), child: const Text('Cancel')),
        FilledButton(
          onPressed: () {
            Navigator.pop(context);
            widget.m.newProject(parent, name.text.trim(), template);
          },
          child: const Text('Create'),
        ),
      ],
    );
  }
}

Future<String?> prompt(BuildContext context, String title, {String initial = '', String action = 'OK'}) {
  final c = TextEditingController(text: initial);
  final dot = initial.lastIndexOf('.');
  c.selection = TextSelection(baseOffset: 0, extentOffset: dot > 0 ? dot : initial.length);
  return showDialog<String>(
    context: context,
    builder: (context) => AlertDialog(
      title: Text(title, style: const TextStyle(fontSize: 16, fontWeight: FontWeight.w600)),
      content: SizedBox(
        width: 360,
        child: TextField(controller: c, autofocus: true, onSubmitted: (v) => Navigator.pop(context, v.trim())),
      ),
      actions: [
        TextButton(onPressed: () => Navigator.pop(context), child: const Text('Cancel')),
        FilledButton(onPressed: () => Navigator.pop(context, c.text.trim()), child: Text(action)),
      ],
    ),
  );
}

Future<void> showSettings(BuildContext context, AppModel m) => showDialog(
  context: context,
  builder: (context) => ListenableBuilder(
    listenable: m,
    builder: (context, _) {
      final k = KT.of(context);
      final e = m.engines;
      return AlertDialog(
        title: const Text('Settings', style: TextStyle(fontSize: 18, fontWeight: FontWeight.w600)),
        content: SizedBox(
          width: 460,
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              _row(
                'Theme',
                SegmentedButton<ThemeMode>(
                  segments: const [
                    ButtonSegment(value: ThemeMode.light, label: Text('Light')),
                    ButtonSegment(value: ThemeMode.dark, label: Text('Dark')),
                    ButtonSegment(value: ThemeMode.system, label: Text('System')),
                  ],
                  selected: {m.themeMode},
                  showSelectedIcon: false,
                  onSelectionChanged: (s) => m.update(() => m.themeMode = s.first),
                ),
              ),
              _row(
                'Editor font size',
                Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    IconButton(
                      onPressed: () => m.update(() => m.fontSize = (m.fontSize - 1).clamp(9, 28)),
                      icon: const Icon(Icons.remove, size: 16),
                    ),
                    Text('${m.fontSize.round()} px'),
                    IconButton(
                      onPressed: () => m.update(() => m.fontSize = (m.fontSize + 1).clamp(9, 28)),
                      icon: const Icon(Icons.add, size: 16),
                    ),
                  ],
                ),
              ),
              _row('Compile on save', Switch(value: m.compileOnSave, onChanged: (v) => m.update(() => m.compileOnSave = v))),
              const Divider(height: 28),
              Text(
                'Engines',
                style: TextStyle(color: k.muted, fontSize: 12, fontWeight: FontWeight.w600),
              ),
              const SizedBox(height: 6),
              _engine(k, 'Typst (built in)', e?['typstVersion'] != null ? 'Typst ${e!['typstVersion']}' : null),
              _engine(k, 'Tectonic (bundled)', e?['tectonicVersion'] as String?),
              _engine(k, 'latexmk (your TeX distribution)', e?['latexmkVersion'] as String?),
            ],
          ),
        ),
        actions: [FilledButton(onPressed: () => Navigator.pop(context), child: const Text('Done'))],
      );
    },
  ),
);

Widget _row(String label, Widget w) => Padding(
  padding: const EdgeInsets.symmetric(vertical: 4),
  child: Row(
    children: [
      Expanded(child: Text(label)),
      w,
    ],
  ),
);

Widget _engine(KT k, String name, String? version) => Padding(
  padding: const EdgeInsets.symmetric(vertical: 3),
  child: Row(
    children: [
      Icon(version != null ? Icons.check_circle : Icons.remove_circle_outline, size: 15, color: version != null ? k.success : k.faint),
      const SizedBox(width: 8),
      Expanded(child: Text(name)),
      Text(version ?? 'not found', style: TextStyle(color: k.muted, fontSize: 12)),
    ],
  ),
);

Future<void> showAbout(BuildContext context) => showDialog(
  context: context,
  builder: (context) {
    final k = KT.of(context);
    void open(String url) => Core.call('openUrl', {'url': url}).catchError((_) => null);
    return AlertDialog(
      content: SizedBox(
        width: 380,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Image.asset('assets/logo.png', width: 72, height: 72),
            const SizedBox(height: 10),
            const Text('KernTeX', style: TextStyle(fontSize: 22, fontWeight: FontWeight.w700)),
            Text('Version $appVersion · Flutter preview', style: TextStyle(color: k.muted)),
            const SizedBox(height: 12),
            const Text(
              'A fast, local editor for LaTeX and Typst.\nFlutter UI on the same Rust core as the desktop app.',
              textAlign: TextAlign.center,
            ),
            const SizedBox(height: 14),
            Wrap(
              spacing: 8,
              children: [
                OutlinedButton(onPressed: () => open(repoUrl), child: const Text('GitHub')),
                OutlinedButton(onPressed: () => open('$repoUrl/issues/new'), child: const Text('Report a problem')),
              ],
            ),
            const SizedBox(height: 12),
            Text('MIT License · Built with Rust, Flutter, Typst, Tectonic and hayro', style: TextStyle(color: k.faint, fontSize: 11)),
          ],
        ),
      ),
      actions: [TextButton(onPressed: () => Navigator.pop(context), child: const Text('Close'))],
    );
  },
);
