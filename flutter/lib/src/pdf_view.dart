// PDF preview. Pages are rasterised by the Rust core (hayro) at exactly the
// on-screen resolution. One render runs at a time; the scheduler always picks
// the visible page closest to the viewport centre, so fast scrolling never
// builds up a queue. Only a handful of bitmaps stay in memory.

import 'dart:async';
import 'dart:ui' as ui;

import 'package:flutter/gestures.dart';
import 'package:flutter/services.dart';
import 'package:material_ui/material_ui.dart';

import 'native.dart';
import 'theme.dart';

class PdfView extends StatefulWidget {
  const PdfView({super.key, required this.path, required this.version, this.compact = false});

  final String path;
  final int version;
  final bool compact; // inline file preview (no toolbar)

  @override
  State<PdfView> createState() => _PdfViewState();
}

class _Bitmap {
  _Bitmap(this.image, this.scale, this.version);
  final ui.Image image;
  final double scale;
  final int version;
}

class _PdfViewState extends State<PdfView> {
  List<Size> _pages = const [];
  String? _error;
  double _zoom = 1; // 1 = fit width
  final _scroll = ScrollController();
  final _bitmaps = <int, _Bitmap>{};
  final _wanted = <int, double>{}; // page -> device scale
  final _lru = <int>[];
  bool _busy = false;
  int _loadedVersion = -1;
  int _page = 1;
  static const _keep = 10;
  static const _gap = 14.0;

  @override
  void initState() {
    super.initState();
    _load();
    _scroll.addListener(_onScroll);
  }

  @override
  void didUpdateWidget(PdfView old) {
    super.didUpdateWidget(old);
    if (old.path != widget.path || old.version != widget.version) _load();
  }

  @override
  void dispose() {
    for (final b in _bitmaps.values) {
      b.image.dispose();
    }
    _scroll.dispose();
    super.dispose();
  }

  Future<void> _load() async {
    final version = widget.version;
    try {
      final info = await Core.quick('pdfInfo', {'path': widget.path}) as Map<String, dynamic>?;
      if (!mounted || version != widget.version) return;
      setState(() {
        _error = info == null ? 'No PDF yet' : null;
        _pages = [for (final p in (info?['pages'] as List? ?? const [])) Size((p[0] as num).toDouble(), (p[1] as num).toDouble())];
        _loadedVersion = version;
        // Old bitmaps stay on screen (no flicker) until the new ones arrive.
        _bitmaps.removeWhere((k, b) {
          if (k < _pages.length) return false;
          b.image.dispose();
          return true;
        });
      });
    } catch (e) {
      if (mounted) setState(() => _error = '$e');
    }
  }

  void _onScroll() {
    if (_pages.isEmpty) return;
    final y = _scroll.offset + 1;
    var acc = 0.0;
    for (var i = 0; i < _pages.length; i++) {
      acc += _pageH(i) + _gap;
      if (acc > y) {
        if (_page != i + 1) setState(() => _page = i + 1);
        return;
      }
    }
  }

  double _width = 600;
  double _pageW(int i) => (_width - 32) * _zoom;
  double _pageH(int i) => _pageW(i) * _pages[i].height / _pages[i].width;

  void _want(int page, double scale) {
    final have = _bitmaps[page];
    if (have != null && have.version == _loadedVersion && (have.scale / scale - 1).abs() < 0.12) return;
    _wanted[page] = scale;
    if (!_busy) scheduleMicrotask(_pump);
  }

  Future<void> _pump() async {
    if (_busy || _wanted.isEmpty || !mounted) return;
    _busy = true;
    try {
      while (_wanted.isNotEmpty && mounted) {
        // Closest to the current page first.
        final page = _wanted.keys.reduce((a, b) => (a - _page + 1).abs() <= (b - _page + 1).abs() ? a : b);
        final scale = _wanted.remove(page)!;
        final version = _loadedVersion;
        final bmp = await Core.renderPage(widget.path, page, scale);
        if (bmp == null || !mounted) continue;
        final c = Completer<ui.Image>();
        ui.decodeImageFromPixels(bmp.pixels, bmp.width, bmp.height, ui.PixelFormat.rgba8888, c.complete);
        final img = await c.future;
        if (!mounted) {
          img.dispose();
          break;
        }
        setState(() {
          _bitmaps.remove(page)?.image.dispose();
          _bitmaps[page] = _Bitmap(img, scale, version);
          _lru
            ..remove(page)
            ..add(page);
          while (_lru.length > _keep) {
            final old = _lru.removeAt(0);
            if ((old - _page + 1).abs() > 2) _bitmaps.remove(old)?.image.dispose();
          }
        });
      }
    } finally {
      _busy = false;
    }
  }

  void _setZoom(double z) => setState(() => _zoom = z.clamp(0.25, 5.0));

  @override
  Widget build(BuildContext context) {
    final k = KT.of(context);
    final dpr = MediaQuery.devicePixelRatioOf(context);
    if (_error != null && _pages.isEmpty) {
      return ColoredBox(
        color: k.pdfBg,
        child: Center(
          child: Text(_error == 'No PDF yet' ? 'The PDF appears here after the first build.' : _error!, style: TextStyle(color: k.muted)),
        ),
      );
    }
    final body = LayoutBuilder(
      builder: (context, box) {
        _width = box.maxWidth;
        return Listener(
          onPointerSignal: (e) {
            if (e is PointerScrollEvent && HardwareKeyboard.instance.isControlPressed) {
              GestureBinding.instance.pointerSignalResolver.register(e, (_) => _setZoom(_zoom * (e.scrollDelta.dy < 0 ? 1.1 : 1 / 1.1)));
            }
          },
          child: Scrollbar(
            controller: _scroll,
            child: ListView.builder(
              controller: _scroll,
              padding: const EdgeInsets.symmetric(vertical: 16),
              itemCount: _pages.length,
              itemBuilder: (context, i) {
                final w = _pageW(i), h = _pageH(i);
                _want(i, w * dpr / _pages[i].width);
                final b = _bitmaps[i];
                return Padding(
                  padding: const EdgeInsets.only(bottom: _gap),
                  child: Center(
                    child: Container(
                      width: w,
                      height: h,
                      decoration: const BoxDecoration(
                        color: Colors.white,
                        boxShadow: [BoxShadow(color: Color(0x22000000), blurRadius: 6, offset: Offset(0, 1))],
                      ),
                      child: b == null
                          ? null
                          : RawImage(image: b.image, width: w, height: h, fit: BoxFit.fill, filterQuality: FilterQuality.medium),
                    ),
                  ),
                );
              },
            ),
          ),
        );
      },
    );
    if (widget.compact) return ColoredBox(color: k.pdfBg, child: body);
    return ColoredBox(
      color: k.pdfBg,
      child: Column(
        children: [
          Container(
            height: 34,
            padding: const EdgeInsets.symmetric(horizontal: 8),
            decoration: BoxDecoration(
              color: k.surface,
              border: Border(bottom: BorderSide(color: k.border)),
            ),
            child: Row(
              children: [
                Text(_pages.isEmpty ? '' : 'Page $_page of ${_pages.length}', style: TextStyle(color: k.muted, fontSize: 12)),
                const Spacer(),
                _btn(Icons.remove, 'Zoom out (Ctrl+wheel)', () => _setZoom(_zoom / 1.2)),
                SizedBox(
                  width: 48,
                  child: Text(
                    '${(_zoom * 100).round()}%',
                    textAlign: TextAlign.center,
                    style: TextStyle(color: k.muted, fontSize: 12),
                  ),
                ),
                _btn(Icons.add, 'Zoom in', () => _setZoom(_zoom * 1.2)),
                _btn(Icons.fit_screen_outlined, 'Fit width', () => _setZoom(1)),
              ],
            ),
          ),
          Expanded(child: body),
        ],
      ),
    );
  }

  Widget _btn(IconData icon, String tip, VoidCallback onTap) => IconButton(
    icon: Icon(icon, size: 16),
    tooltip: tip,
    onPressed: onTap,
    visualDensity: VisualDensity.compact,
    constraints: const BoxConstraints(minWidth: 28, minHeight: 28),
    padding: EdgeInsets.zero,
  );
}
