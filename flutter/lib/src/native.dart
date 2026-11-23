// Bindings to the Rust core (flutter/native, built by hook/build.dart).
//
// Two long-lived worker isolates keep the UI thread free: one runs builds and
// other slow calls, the other rasterises PDF pages — so a running build never
// delays scrolling the preview. Cheap calls (reading a file) run inline.

// ignore_for_file: non_constant_identifier_names
@DefaultAsset('package:kerntex/src/native.dart')
library;

import 'dart:async';
import 'dart:convert';
import 'dart:ffi';
import 'dart:isolate';
import 'dart:typed_data';

import 'package:ffi/ffi.dart';

@Native<Pointer<Utf8> Function(Pointer<Utf8>, Pointer<Utf8>)>()
external Pointer<Utf8> kt_call(Pointer<Utf8> method, Pointer<Utf8> args);

@Native<Void Function(Pointer<Utf8>)>(isLeaf: true)
external void kt_free(Pointer<Utf8> p);

@Native<Uint8 Function()>()
external int kt_cancel();

@Native<Pointer<Utf8> Function()>(isLeaf: true)
external Pointer<Utf8> kt_progress();

@Native<Pointer<Uint8> Function(Pointer<Utf8>, Uint32, Float, Pointer<Uint32>, Pointer<Uint32>)>()
external Pointer<Uint8> kt_pdf_render(Pointer<Utf8> path, int page, double scale, Pointer<Uint32> w, Pointer<Uint32> h);

@Native<Void Function(Pointer<Uint8>, Size)>(isLeaf: true)
external void kt_buf_free(Pointer<Uint8> p, int len);

/// An error reported by the core: a title, plus optional detail and fix.
class CoreError implements Exception {
  CoreError(Map<String, dynamic> e)
    : source = e['source'] as String? ?? 'app',
      title = e['title'] as String? ?? 'Something went wrong',
      detail = e['detail'] as String?,
      hint = e['hint'] as String?;
  final String source;
  final String title;
  final String? detail;
  final String? hint;

  @override
  String toString() => [title, detail, hint].whereType<String>().join(' — ');
}

/// Raw call: returns the JSON reply string (`{"ok":…}` / `{"err":…}`).
String _rawCall(String method, Map<String, Object?> args) {
  final m = method.toNativeUtf8();
  final a = jsonEncode(args).toNativeUtf8();
  try {
    final r = kt_call(m, a);
    final s = r.toDartString();
    kt_free(r);
    return s;
  } finally {
    malloc.free(m);
    malloc.free(a);
  }
}

dynamic _unwrap(String reply) {
  final v = jsonDecode(reply) as Map<String, dynamic>;
  if (v.containsKey('err')) throw CoreError(v['err'] as Map<String, dynamic>);
  return v['ok'];
}

/// A rendered page: straight RGBA, `width * height * 4` bytes.
class PageBitmap {
  PageBitmap(this.width, this.height, this.pixels);
  final int width;
  final int height;
  final Uint8List pixels;
}

class _Worker {
  _Worker._(this._send);
  final SendPort _send;
  final _pending = <int, Completer<Object?>>{};
  int _next = 0;

  static Future<_Worker> spawn() async {
    final replies = ReceivePort();
    final ready = Completer<SendPort>();
    late _Worker w;
    replies.listen((msg) {
      if (msg is SendPort) {
        ready.complete(msg);
      } else {
        final (int id, Object? value) = msg as (int, Object?);
        w._pending.remove(id)?.complete(value);
      }
    });
    await Isolate.spawn(_main, replies.sendPort, debugName: 'kerntex-core');
    w = _Worker._(await ready.future);
    return w;
  }

  Future<Object?> run(Object job) {
    final id = _next++;
    final c = Completer<Object?>();
    _pending[id] = c;
    _send.send((id, job));
    return c.future;
  }

  static void _main(SendPort replies) {
    final jobs = ReceivePort();
    replies.send(jobs.sendPort);
    jobs.listen((msg) {
      final (int id, Object job) = msg as (int, Object);
      replies.send((id, _perform(job)));
    });
  }

  static Object? _perform(Object job) {
    switch (job) {
      case (String method, Map<String, Object?> args):
        return _rawCall(method, args);
      case (String path, int page, double scale):
        final w = calloc<Uint32>(), h = calloc<Uint32>(), p = path.toNativeUtf8();
        try {
          final buf = kt_pdf_render(p, page, scale, w, h);
          if (buf == nullptr) return null;
          final len = w.value * h.value * 4;
          final data = TransferableTypedData.fromList([buf.asTypedList(len)]);
          kt_buf_free(buf, len);
          return (w.value, h.value, data);
        } finally {
          calloc.free(w);
          calloc.free(h);
          malloc.free(p);
        }
    }
    return null;
  }
}

/// The KernTeX core.
class Core {
  Core._();
  static _Worker? _build, _render;

  static Future<void> init() async {
    _build ??= await _Worker.spawn();
    _render ??= await _Worker.spawn();
  }

  /// Runs on the UI isolate — only for calls that touch one small file.
  static dynamic now(String method, [Map<String, Object?> args = const {}]) => _unwrap(_rawCall(method, args));

  /// Runs on the build worker.
  static Future<dynamic> call(String method, [Map<String, Object?> args = const {}]) async =>
      _unwrap(await _build!.run((method, args)) as String);

  /// Runs on the render worker — for quick calls that must not wait for a build.
  static Future<dynamic> quick(String method, [Map<String, Object?> args = const {}]) async =>
      _unwrap(await _render!.run((method, args)) as String);

  static bool cancel() => kt_cancel() != 0;

  static Map<String, dynamic>? progress() {
    final p = kt_progress();
    final s = p.toDartString();
    kt_free(p);
    return jsonDecode(s) as Map<String, dynamic>?;
  }

  static Future<PageBitmap?> renderPage(String pdf, int page, double scale) async {
    final r = await _render!.run((pdf, page, scale));
    if (r == null) return null;
    final (int w, int h, TransferableTypedData d) = r as (int, int, TransferableTypedData);
    return PageBitmap(w, h, d.materialize().asUint8List());
  }
}
