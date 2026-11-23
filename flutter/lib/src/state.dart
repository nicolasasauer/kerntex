// App state: one ChangeNotifier, no state-management package.

import 'dart:async';

import 'package:material_ui/material_ui.dart';

import 'editor/code_controller.dart';
import 'native.dart';

class Node {
  Node(Map<String, dynamic> j)
    : name = j['name'] as String,
      path = j['path'] as String,
      isDir = j['isDir'] as bool,
      generated = j['generated'] as bool? ?? false,
      children = [for (final c in (j['children'] as List? ?? const [])) Node(c as Map<String, dynamic>)];
  final String name, path;
  final bool isDir, generated;
  final List<Node> children;
}

class Recent {
  Recent(this.root, this.name, this.openedAt);
  final String root, name;
  final int openedAt;
  Map<String, Object> toJson() => {'root': root, 'name': name, 'openedAt': openedAt};
}

class Doc {
  Doc(this.path, String text) : saved = text, controller = CodeController(text: text, lang: langFor(path));
  final String path;
  String saved;
  final CodeController controller;
  bool get dirty => controller.text != saved;
  String get name => path.split('/').last;
}

enum BuildState { idle, running, done }

const textExtensions = {
  'tex',
  'typ',
  'bib',
  'sty',
  'cls',
  'txt',
  'md',
  'bbx',
  'cbx',
  'ltx',
  'dtx',
  'ins',
  'cfg',
  'csv',
  'json',
  'yaml',
  'yml',
  'latexmkrc',
};
const imageExtensions = {'png', 'jpg', 'jpeg', 'gif', 'webp', 'bmp'};

String ext(String path) {
  final n = path.split('/').last;
  final i = n.lastIndexOf('.');
  return i < 0 ? n.toLowerCase() : n.substring(i + 1).toLowerCase();
}

class AppModel extends ChangeNotifier {
  // ── Persistent preferences ──────────────────────────────────────────────
  ThemeMode themeMode = ThemeMode.system;
  double fontSize = 14;
  bool compileOnSave = true;
  bool treeOpen = true, pdfOpen = true, problemsOpen = false;
  double treeWidth = 230, pdfFraction = 0.45, problemsHeight = 220;
  List<Recent> recents = [];

  // ── Session ─────────────────────────────────────────────────────────────
  Map<String, dynamic>? engines;
  String? root, name, mainFile;
  Map<String, dynamic> projectSettings = {};
  Map<String, dynamic> hints = {};
  List<Node> tree = [];
  final docs = <Doc>[];
  String? activePath;
  String? previewPath; // an image / PDF opened from the tree

  BuildState build = BuildState.idle;
  String? progress;
  Map<String, dynamic>? result;
  List<Map<String, dynamic>> diagnostics = [];
  String? pdfPath;
  int pdfVersion = 0;
  String bottomTab = 'problems';
  String? toast;
  Timer? _toastTimer, _progressTimer, _saveTimer;

  Doc? get active => docs.where((d) => d.path == activePath).firstOrNull;
  bool get isTypst => (mainFile ?? '').toLowerCase().endsWith('.typ');

  String get engine {
    if (isTypst) return 'typst';
    final e = projectSettings['engine'] as String?;
    if (e != null) return e;
    if (hints['hasLatexmkrc'] == true) return 'latexmk';
    if (engines?['tectonic'] == null && engines?['latexmk'] != null) return 'latexmk';
    return 'tectonic';
  }

  String get flavor => (projectSettings['flavor'] ?? hints['flavor'] ?? 'pdflatex') as String;

  int count(String severity) => diagnostics.where((d) => d['severity'] == severity).length;

  Future<void> init() async {
    final s = Core.now('loadState') as Map<String, dynamic>?;
    if (s != null) {
      themeMode = ThemeMode.values.byName(s['theme'] as String? ?? 'system');
      fontSize = (s['fontSize'] as num?)?.toDouble() ?? fontSize;
      compileOnSave = s['compileOnSave'] as bool? ?? compileOnSave;
      treeOpen = s['treeOpen'] as bool? ?? treeOpen;
      pdfOpen = s['pdfOpen'] as bool? ?? pdfOpen;
      treeWidth = (s['treeWidth'] as num?)?.toDouble() ?? treeWidth;
      pdfFraction = (s['pdfFraction'] as num?)?.toDouble() ?? pdfFraction;
      problemsHeight = (s['problemsHeight'] as num?)?.toDouble() ?? problemsHeight;
      recents = [
        for (final r in (s['recent'] as List? ?? const [])) Recent(r['root'] as String, r['name'] as String, r['openedAt'] as int),
      ];
    }
    notifyListeners();
    engines = await Core.call('detectEngines') as Map<String, dynamic>;
    notifyListeners();
  }

  /// Debounced, so dragging a splitter doesn't write on every frame.
  void persist() {
    _saveTimer?.cancel();
    _saveTimer = Timer(const Duration(milliseconds: 400), () {
      Core.now('saveState', {
        'state': {
          'theme': themeMode.name,
          'fontSize': fontSize,
          'compileOnSave': compileOnSave,
          'treeOpen': treeOpen,
          'pdfOpen': pdfOpen,
          'treeWidth': treeWidth,
          'pdfFraction': pdfFraction,
          'problemsHeight': problemsHeight,
          'recent': [for (final r in recents) r.toJson()],
        },
      });
    });
  }

  void update(VoidCallback f) {
    f();
    notifyListeners();
    persist();
  }

  void showToast(String text) {
    toast = text;
    notifyListeners();
    _toastTimer?.cancel();
    _toastTimer = Timer(const Duration(seconds: 4), () {
      toast = null;
      notifyListeners();
    });
  }

  void fail(Object e) => showToast(e is CoreError ? [e.title, e.hint ?? e.detail].whereType<String>().join(' — ') : '$e');

  // ── Projects ────────────────────────────────────────────────────────────

  Future<void> openProject(String path) async {
    try {
      final info = await Core.call('openProject', {'root': path}) as Map<String, dynamic>;
      _closeDocs();
      root = info['root'] as String;
      name = info['name'] as String;
      mainFile = info['mainFile'] as String?;
      projectSettings = (info['settings'] as Map<String, dynamic>?) ?? {};
      hints = (info['hints'] as Map<String, dynamic>?) ?? {};
      tree = [for (final n in info['tree'] as List) Node(n as Map<String, dynamic>)];
      recents.removeWhere((r) => r.root == root);
      recents.insert(0, Recent(root!, name!, DateTime.now().millisecondsSinceEpoch));
      if (recents.length > 12) recents.removeRange(12, recents.length);
      persist();
      final existing = mainFile == null ? null : Core.now('pdfPath', {'root': root, 'mainFile': mainFile}) as String;
      pdfPath = existing;
      pdfVersion++;
      if (mainFile != null) await openFile(mainFile!);
      notifyListeners();
      if (mainFile != null) unawaited(compile());
    } catch (e) {
      fail(e);
    }
  }

  void _closeDocs() {
    for (final d in docs) {
      d.controller.dispose();
    }
    docs.clear();
    activePath = previewPath = null;
    diagnostics = [];
    result = null;
    build = BuildState.idle;
    pdfPath = null;
  }

  Future<void> closeProject() async {
    await saveAll();
    _closeDocs();
    root = name = mainFile = null;
    tree = [];
    notifyListeners();
  }

  void removeRecent(String r) => update(() => recents.removeWhere((x) => x.root == r));

  Future<void> refreshTree() async {
    if (root == null) return;
    try {
      tree = [
        for (final n in await Core.call('readTree', {'root': root}) as List) Node(n as Map<String, dynamic>),
      ];
      notifyListeners();
    } catch (e) {
      fail(e);
    }
  }

  Future<void> newProject(String parent, String name, String template) async {
    try {
      await openProject(await Core.call('newProject', {'parent': parent, 'name': name, 'template': template}) as String);
    } catch (e) {
      fail(e);
    }
  }

  Future<void> openWelcome(String kind, {bool fresh = false}) async {
    try {
      await openProject(await Core.call('welcome', {'kind': kind, 'fresh': fresh}) as String);
    } catch (e) {
      fail(e);
    }
  }

  Future<void> importZip(String zip) async {
    try {
      final docsDir = Core.now('documentsDir') as String;
      await openProject(await Core.call('importZip', {'zip': zip, 'destParent': docsDir}) as String);
    } catch (e) {
      fail(e);
    }
  }

  Future<void> setMain(String path) async {
    mainFile = path;
    projectSettings = {...projectSettings, 'mainFile': path};
    try {
      Core.now('saveSettings', {'root': root, 'settings': projectSettings});
    } catch (e) {
      fail(e);
    }
    pdfPath = Core.now('pdfPath', {'root': root, 'mainFile': path}) as String;
    pdfVersion++;
    notifyListeners();
    unawaited(compile());
  }

  // ── Files ───────────────────────────────────────────────────────────────

  Future<void> openFile(String path) async {
    final e = ext(path);
    if (imageExtensions.contains(e) || e == 'pdf') {
      previewPath = path;
      activePath = null;
      notifyListeners();
      return;
    }
    previewPath = null;
    if (docs.every((d) => d.path != path)) {
      try {
        docs.add(Doc(path, Core.now('readFile', {'root': root, 'path': path}) as String));
      } catch (err) {
        fail(err);
        return;
      }
    }
    activePath = path;
    _applyDiagnostics();
    notifyListeners();
  }

  void closeDoc(Doc d) {
    if (d.dirty) _write(d);
    final i = docs.indexOf(d);
    docs.remove(d);
    if (activePath == d.path) activePath = docs.isEmpty ? null : docs[i.clamp(0, docs.length - 1)].path;
    d.controller.dispose();
    notifyListeners();
  }

  bool _write(Doc d) {
    try {
      final text = d.controller.text;
      Core.now('writeFile', {'root': root, 'path': d.path, 'content': text});
      d.saved = text;
      return true;
    } catch (e) {
      fail(e);
      return false;
    }
  }

  Future<void> saveAll() async {
    for (final d in docs) {
      if (d.dirty) _write(d);
    }
    notifyListeners();
  }

  Future<void> save() async {
    await saveAll();
    if (compileOnSave) await compile();
  }

  Future<void> createEntry(String dir, String name, {required bool folder}) async {
    final path = dir.isEmpty ? name : '$dir/$name';
    try {
      Core.now(folder ? 'createDir' : 'createFile', {'root': root, 'path': path});
      await refreshTree();
      if (!folder) await openFile(path);
    } catch (e) {
      fail(e);
    }
  }

  Future<void> rename(String from, String to) async {
    try {
      Core.now('rename', {'root': root, 'from': from, 'to': to});
      for (final d in [...docs]) {
        if (d.path == from || d.path.startsWith('$from/')) {
          _write(d);
          docs.remove(d);
          d.controller.dispose();
        }
      }
      if (mainFile == from) await setMain(to);
      await refreshTree();
    } catch (e) {
      fail(e);
    }
  }

  Future<void> trash(String path) async {
    try {
      Core.now('trash', {'root': root, 'path': path});
      docs.removeWhere((d) => d.path == path || d.path.startsWith('$path/'));
      if (activePath != null && docs.every((d) => d.path != activePath)) activePath = docs.firstOrNull?.path;
      await refreshTree();
    } catch (e) {
      fail(e);
    }
  }

  Future<void> importFiles(String dir, List<String> sources) async {
    try {
      final added = await Core.call('importFiles', {'root': root, 'dir': dir, 'sources': sources}) as List;
      await refreshTree();
      showToast(added.length == 1 ? 'Added ${added.first}' : 'Added ${added.length} files');
    } catch (e) {
      fail(e);
    }
  }

  // ── Build ───────────────────────────────────────────────────────────────

  Future<void> compile() async {
    final r = root, main = mainFile;
    if (r == null || main == null) {
      showToast('Choose a main file first: right-click a .tex or .typ file → Set as main file');
      return;
    }
    await saveAll();
    build = BuildState.running;
    progress = engine == 'tectonic' && engines?['tectonicCacheReady'] == false
        ? 'Downloading LaTeX packages (first build only)…'
        : 'Compiling…';
    notifyListeners();
    _progressTimer?.cancel();
    _progressTimer = Timer.periodic(const Duration(milliseconds: 250), (_) {
      final p = Core.progress();
      if (p != null) {
        progress = [p['stage'], p['detail']].whereType<String>().join(' · ');
        notifyListeners();
      }
    });
    try {
      final res = await Core.call('compile', {
        'request': {'root': r, 'mainFile': main, 'engine': engine == 'typst' ? 'tectonic' : engine, 'flavor': flavor},
      }) as Map<String, dynamic>;
      if (root != r) return; // project changed meanwhile
      if (res['canceled'] == true) {
        build = BuildState.idle;
        return;
      }
      result = res;
      diagnostics = [for (final d in res['diagnostics'] as List) d as Map<String, dynamic>];
      final pdf = res['pdfPath'] as String?;
      if (pdf != null) {
        pdfPath = pdf;
        pdfVersion++;
      }
      if (engine == 'tectonic' && engines?['tectonicCacheReady'] == false && res['ok'] == true) {
        engines = {...?engines, 'tectonicCacheReady': true};
      }
      if (count('error') > 0 && !problemsOpen) problemsOpen = true;
      build = BuildState.done;
      _applyDiagnostics();
    } catch (e) {
      build = BuildState.done;
      result = null;
      final err = e is CoreError ? e : null;
      diagnostics = [
        {'severity': 'error', 'source': 'KernTeX', 'message': err?.title ?? '$e', 'hint': err?.hint ?? err?.detail},
      ];
      problemsOpen = true;
    } finally {
      _progressTimer?.cancel();
      progress = null;
      notifyListeners();
    }
  }

  void cancelCompile() => Core.cancel();

  void _applyDiagnostics() {
    for (final d in docs) {
      d.controller.setDiagnostics([
        for (final x in diagnostics)
          if (x['file'] == d.path) x,
      ]);
    }
  }
}
