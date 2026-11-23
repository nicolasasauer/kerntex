// Builds the Rust core (`native/`, which includes `../src-tauri/src/core`)
// with cargo and bundles it with the app. `flutter run` / `flutter build`
// call this automatically — no CMake or manual copying.
//
// Options (pubspec.yaml → hooks.user_defines.kerntex):
//   cargo_profile: release (default, LTO) | fast (no LTO, for quick local builds)
// Hooks run with a filtered environment; CARGO_* and RUSTUP_* pass through,
// so CARGO_TARGET_DIR (e.g. a CI cache) is honoured.

import 'dart:io';

import 'package:code_assets/code_assets.dart';
import 'package:hooks/hooks.dart';

final _triples = {
  (OS.linux, Architecture.x64): 'x86_64-unknown-linux-gnu',
  (OS.linux, Architecture.arm64): 'aarch64-unknown-linux-gnu',
  (OS.windows, Architecture.x64): 'x86_64-pc-windows-msvc',
  (OS.windows, Architecture.arm64): 'aarch64-pc-windows-msvc',
  (OS.macOS, Architecture.x64): 'x86_64-apple-darwin',
  (OS.macOS, Architecture.arm64): 'aarch64-apple-darwin',
  (OS.android, Architecture.arm64): 'aarch64-linux-android',
};

void main(List<String> args) async {
  await build(args, (input, output) async {
    if (!input.config.buildCodeAssets) return;
    final code = input.config.code;
    final triple = _triples[(code.targetOS, code.targetArchitecture)];
    if (triple == null) {
      throw UnsupportedError('KernTeX core: no Rust target for ${code.targetOS}/${code.targetArchitecture}');
    }
    final manifest = input.packageRoot.resolve('native/Cargo.toml');
    final profile = input.userDefines['cargo_profile'] ?? 'release';
    if (profile is! String || !const {'release', 'fast'}.contains(profile)) {
      throw const FormatException('hooks.user_defines.kerntex.cargo_profile must be "release" or "fast"');
    }
    final envTarget = Platform.environment['CARGO_TARGET_DIR'];
    final targetDir = envTarget != null && envTarget.isNotEmpty
        ? Directory(envTarget).absolute.uri.resolve('./')
        : input.outputDirectoryShared.resolve('cargo/');
    final home = Platform.environment['HOME'] ?? Platform.environment['USERPROFILE'] ?? '';
    final local = File('$home/.cargo/bin/cargo${Platform.isWindows ? '.exe' : ''}');
    final cargo = local.existsSync() ? local.path : 'cargo';

    final result = await Process.run(cargo, [
      'build',
      '--profile',
      profile,
      '--locked',
      '--manifest-path',
      manifest.toFilePath(),
      '--target',
      triple,
      '--target-dir',
      targetDir.toFilePath(),
    ]);
    if (result.exitCode != 0) {
      throw Exception('cargo build failed (${result.exitCode}):\n${result.stderr}');
    }

    final lib = targetDir.resolve('$triple/$profile/${code.targetOS.dylibFileName('kerntex_native')}');
    output.assets.code.add(CodeAsset(package: input.packageName, name: 'src/native.dart', linkMode: DynamicLoadingBundled(), file: lib));

    // Rebuild whenever the Rust sources, the shared core or its templates change.
    final repo = input.packageRoot.resolve('../');
    for (final dir in ['flutter/native/src/', 'src-tauri/src/core/', 'src-tauri/templates/']) {
      final d = Directory.fromUri(repo.resolve(dir));
      if (!d.existsSync()) continue;
      for (final f in d.listSync(recursive: true).whereType<File>()) {
        output.dependencies.add(f.uri);
      }
    }
    output.dependencies.add(manifest);
    output.dependencies.add(input.packageRoot.resolve('native/Cargo.lock'));
  });
}
