// Design tokens — the same values as src/app.css, so both UIs look alike.

import 'package:material_ui/material_ui.dart';

@immutable
class KT extends ThemeExtension<KT> {
  const KT({
    required this.bg,
    required this.surface,
    required this.surface2,
    required this.editorBg,
    required this.pdfBg,
    required this.border,
    required this.borderStrong,
    required this.text,
    required this.muted,
    required this.faint,
    required this.accent,
    required this.accentSoft,
    required this.onAccent,
    required this.danger,
    required this.dangerSoft,
    required this.warning,
    required this.warningSoft,
    required this.info,
    required this.success,
    required this.hover,
    required this.selection,
    required this.activeLine,
    required this.synCommand,
    required this.synKeyword,
    required this.synAtom,
    required this.synNumber,
    required this.synMath,
    required this.synComment,
    required this.synBracket,
    required this.synString,
  });

  final Color bg, surface, surface2, editorBg, pdfBg, border, borderStrong;
  final Color text, muted, faint, accent, accentSoft, onAccent;
  final Color danger, dangerSoft, warning, warningSoft, info, success, hover, selection, activeLine;
  final Color synCommand, synKeyword, synAtom, synNumber, synMath, synComment, synBracket, synString;

  static const light = KT(
    bg: Color(0xfff6f7f9),
    surface: Color(0xffffffff),
    surface2: Color(0xfff1f3f6),
    editorBg: Color(0xffffffff),
    pdfBg: Color(0xffe9ecf1),
    border: Color(0xffe2e5ea),
    borderStrong: Color(0xffcfd4dc),
    text: Color(0xff1d2330),
    muted: Color(0xff5b6474),
    faint: Color(0xff98a0ae),
    accent: Color(0xff3b5bdb),
    accentSoft: Color(0xffe7ecfd),
    onAccent: Color(0xffffffff),
    danger: Color(0xffd6336c),
    dangerSoft: Color(0xfffde8ef),
    warning: Color(0xffd9822b),
    warningSoft: Color(0xfffdf1e3),
    info: Color(0xff1c7ed6),
    success: Color(0xff2f9e44),
    hover: Color(0x0f1e283c),
    selection: Color(0xffd4defc),
    activeLine: Color(0xfff6f8fc),
    synCommand: Color(0xff3b5bdb),
    synKeyword: Color(0xff9c36b5),
    synAtom: Color(0xffc2255c),
    synNumber: Color(0xffe8590c),
    synMath: Color(0xff0b7285),
    synComment: Color(0xff8c95a4),
    synBracket: Color(0xff5b6474),
    synString: Color(0xff2b8a3e),
  );

  static const dark = KT(
    bg: Color(0xff13161c),
    surface: Color(0xff1a1e26),
    surface2: Color(0xff20252f),
    editorBg: Color(0xff171b22),
    pdfBg: Color(0xff0f1216),
    border: Color(0xff2a303b),
    borderStrong: Color(0xff39414f),
    text: Color(0xffe3e7ee),
    muted: Color(0xff9aa3b2),
    faint: Color(0xff636c7b),
    accent: Color(0xff7b93f6),
    accentSoft: Color(0xff27304d),
    onAccent: Color(0xff0f1322),
    danger: Color(0xfff06595),
    dangerSoft: Color(0xff3a1f2b),
    warning: Color(0xfff5a65b),
    warningSoft: Color(0xff3a2c1c),
    info: Color(0xff4dabf7),
    success: Color(0xff51cf66),
    hover: Color(0x14ffffff),
    selection: Color(0xff2f3b63),
    activeLine: Color(0xff1c212a),
    synCommand: Color(0xff8ea2f8),
    synKeyword: Color(0xffda77f2),
    synAtom: Color(0xfff783ac),
    synNumber: Color(0xffffa94d),
    synMath: Color(0xff66d9e8),
    synComment: Color(0xff6c7586),
    synBracket: Color(0xff9aa3b2),
    synString: Color(0xff8ce99a),
  );

  static KT of(BuildContext c) => Theme.of(c).extension<KT>()!;

  @override
  KT copyWith() => this;

  @override
  KT lerp(KT? other, double t) => t < 0.5 ? this : (other ?? this);
}

// First installed family wins: Windows ships Cascadia Mono / Consolas, Linux
// DejaVu Sans Mono, macOS Menlo.
const monoFamily = 'JetBrains Mono';
const monoFallback = ['Cascadia Mono', 'Consolas', 'Menlo', 'DejaVu Sans Mono', 'Liberation Mono', 'Noto Sans Mono', 'monospace'];

ThemeData buildTheme(Brightness b) {
  final k = b == Brightness.dark ? KT.dark : KT.light;
  final base = ThemeData(
    brightness: b,
    useMaterial3: true,
    colorScheme: ColorScheme.fromSeed(
      seedColor: k.accent,
      brightness: b,
    ).copyWith(primary: k.accent, onPrimary: k.onAccent, surface: k.surface, onSurface: k.text, error: k.danger, outline: k.border),
    scaffoldBackgroundColor: k.bg,
    dividerColor: k.border,
    splashFactory: NoSplash.splashFactory,
    visualDensity: VisualDensity.compact,
    extensions: [k],
  );
  return base.copyWith(
    // Slightly denser text than Material's defaults (sizes are null in tests).
    textTheme: base.textTheme.bodyMedium?.fontSize == null
        ? base.textTheme.apply(bodyColor: k.text, displayColor: k.text)
        : base.textTheme.apply(bodyColor: k.text, displayColor: k.text, fontSizeFactor: 0.93),
    tooltipTheme: TooltipThemeData(
      waitDuration: const Duration(milliseconds: 500),
      textStyle: TextStyle(color: k.surface, fontSize: 12),
      decoration: BoxDecoration(color: k.text, borderRadius: BorderRadius.circular(6)),
    ),
    dialogTheme: DialogThemeData(
      backgroundColor: k.surface,
      surfaceTintColor: Colors.transparent,
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(12)),
    ),
    popupMenuTheme: PopupMenuThemeData(
      color: k.surface,
      surfaceTintColor: Colors.transparent,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(8),
        side: BorderSide(color: k.border),
      ),
      textStyle: TextStyle(color: k.text, fontSize: 13),
    ),
    inputDecorationTheme: InputDecorationTheme(
      isDense: true,
      filled: true,
      fillColor: k.surface,
      contentPadding: const EdgeInsets.symmetric(horizontal: 10, vertical: 10),
      border: OutlineInputBorder(
        borderRadius: BorderRadius.circular(6),
        borderSide: BorderSide(color: k.border),
      ),
      enabledBorder: OutlineInputBorder(
        borderRadius: BorderRadius.circular(6),
        borderSide: BorderSide(color: k.border),
      ),
      focusedBorder: OutlineInputBorder(
        borderRadius: BorderRadius.circular(6),
        borderSide: BorderSide(color: k.accent, width: 1.5),
      ),
    ),
    filledButtonTheme: FilledButtonThemeData(
      style: FilledButton.styleFrom(
        backgroundColor: k.accent,
        foregroundColor: k.onAccent,
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(6)),
        padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 12),
      ),
    ),
    outlinedButtonTheme: OutlinedButtonThemeData(
      style: OutlinedButton.styleFrom(
        foregroundColor: k.text,
        side: BorderSide(color: k.border),
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(6)),
        padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 12),
      ),
    ),
  );
}
