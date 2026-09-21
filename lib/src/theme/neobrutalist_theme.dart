import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

/// Shared neobrutalist design language for VrV Desk.
///
/// Flat saturated color, near-square geometry, a 3 px ink outline, and hard
/// offset shadows replace translucent surfaces and blurred elevation.
abstract final class NeobrutalTheme {
  static const Color ink = Color(0xFF111111);
  static const Color paper = Color(0xFFFFF7E8);
  static const Color surface = Color(0xFFFFFFFF);
  static const Color yellow = Color(0xFFFFD447);
  static const Color cyan = Color(0xFF70D6FF);
  static const Color pink = Color(0xFFFF70A6);
  static const Color mint = Color(0xFF7BF1A8);
  static const Color danger = Color(0xFFFF5C5C);
  static const Color muted = Color(0xFF5B5B5B);

  static const double borderWidth = 3;
  static const double compactBorderWidth = 2;
  static const double radius = 6;
  static const Offset shadowOffset = Offset(4, 4);

  static List<BoxShadow> hardShadow({
    Color color = ink,
    Offset offset = shadowOffset,
  }) => <BoxShadow>[BoxShadow(color: color, offset: offset, blurRadius: 0)];

  static BoxDecoration panel({
    Color color = surface,
    Color borderColor = ink,
    bool shadow = true,
    double cornerRadius = radius,
  }) => BoxDecoration(
    color: color,
    border: Border.all(color: borderColor, width: borderWidth),
    borderRadius: BorderRadius.circular(cornerRadius),
    boxShadow: shadow ? hardShadow() : null,
  );

  static BoxDecoration compactPanel({
    Color color = paper,
    Color borderColor = ink,
    bool shadow = true,
  }) => BoxDecoration(
    color: color,
    border: Border.all(color: borderColor, width: compactBorderWidth),
    borderRadius: BorderRadius.circular(4),
    boxShadow: shadow ? hardShadow(offset: const Offset(3, 3)) : null,
  );

  static ThemeData get light {
    final scheme =
        ColorScheme.fromSeed(
          seedColor: yellow,
          brightness: Brightness.light,
        ).copyWith(
          primary: yellow,
          onPrimary: ink,
          primaryContainer: cyan,
          onPrimaryContainer: ink,
          secondary: cyan,
          onSecondary: ink,
          secondaryContainer: pink,
          onSecondaryContainer: ink,
          tertiary: mint,
          onTertiary: ink,
          tertiaryContainer: mint,
          onTertiaryContainer: ink,
          error: danger,
          onError: ink,
          errorContainer: danger,
          onErrorContainer: ink,
          surface: surface,
          onSurface: ink,
          surfaceContainerHighest: paper,
          outline: ink,
          outlineVariant: ink,
          shadow: ink,
          scrim: ink,
          inverseSurface: ink,
          onInverseSurface: paper,
          inversePrimary: yellow,
        );

    const textTheme = TextTheme(
      displaySmall: TextStyle(
        color: ink,
        fontSize: 34,
        height: 1.05,
        fontWeight: FontWeight.w900,
        letterSpacing: -1,
      ),
      headlineMedium: TextStyle(
        color: ink,
        fontSize: 24,
        height: 1.1,
        fontWeight: FontWeight.w900,
        letterSpacing: -0.4,
      ),
      titleLarge: TextStyle(
        color: ink,
        fontSize: 20,
        height: 1.15,
        fontWeight: FontWeight.w900,
      ),
      titleMedium: TextStyle(
        color: ink,
        fontSize: 16,
        height: 1.2,
        fontWeight: FontWeight.w800,
      ),
      bodyLarge: TextStyle(
        color: ink,
        fontSize: 16,
        height: 1.35,
        fontWeight: FontWeight.w600,
      ),
      bodyMedium: TextStyle(
        color: ink,
        fontSize: 14,
        height: 1.35,
        fontWeight: FontWeight.w500,
      ),
      bodySmall: TextStyle(
        color: muted,
        fontSize: 12,
        height: 1.3,
        fontWeight: FontWeight.w600,
      ),
      labelLarge: TextStyle(
        color: ink,
        fontSize: 14,
        fontWeight: FontWeight.w900,
        letterSpacing: 0.2,
      ),
    );

    const outlinedShape = RoundedRectangleBorder(
      side: BorderSide(color: ink, width: borderWidth),
      borderRadius: BorderRadius.all(Radius.circular(radius)),
    );
    const compactShape = RoundedRectangleBorder(
      side: BorderSide(color: ink, width: compactBorderWidth),
      borderRadius: BorderRadius.all(Radius.circular(4)),
    );

    final buttonStyle = ButtonStyle(
      minimumSize: const WidgetStatePropertyAll(Size(48, 48)),
      padding: const WidgetStatePropertyAll(
        EdgeInsets.symmetric(horizontal: 18, vertical: 12),
      ),
      elevation: const WidgetStatePropertyAll(0),
      shadowColor: const WidgetStatePropertyAll(Colors.transparent),
      foregroundColor: const WidgetStatePropertyAll(ink),
      textStyle: const WidgetStatePropertyAll(
        TextStyle(fontSize: 14, fontWeight: FontWeight.w900),
      ),
      shape: const WidgetStatePropertyAll(outlinedShape),
      side: const WidgetStatePropertyAll(
        BorderSide(color: ink, width: borderWidth),
      ),
      overlayColor: WidgetStateProperty.resolveWith((states) {
        if (states.contains(WidgetState.pressed)) return pink;
        if (states.contains(WidgetState.hovered)) return cyan;
        return null;
      }),
    );

    return ThemeData(
      useMaterial3: true,
      brightness: Brightness.light,
      colorScheme: scheme,
      scaffoldBackgroundColor: paper,
      canvasColor: paper,
      splashFactory: InkRipple.splashFactory,
      visualDensity: VisualDensity.standard,
      materialTapTargetSize: MaterialTapTargetSize.padded,
      textTheme: textTheme,
      primaryTextTheme: textTheme,
      iconTheme: const IconThemeData(color: ink),
      appBarTheme: const AppBarTheme(
        backgroundColor: yellow,
        foregroundColor: ink,
        centerTitle: false,
        elevation: 0,
        scrolledUnderElevation: 0,
        titleTextStyle: TextStyle(
          color: ink,
          fontSize: 20,
          fontWeight: FontWeight.w900,
          letterSpacing: -0.2,
        ),
        shape: Border(
          bottom: BorderSide(color: ink, width: borderWidth),
        ),
      ),
      cardTheme: const CardThemeData(
        color: surface,
        surfaceTintColor: Colors.transparent,
        elevation: 0,
        margin: EdgeInsets.zero,
        shape: outlinedShape,
      ),
      dialogTheme: const DialogThemeData(
        backgroundColor: paper,
        surfaceTintColor: Colors.transparent,
        elevation: 0,
        shape: outlinedShape,
        titleTextStyle: TextStyle(
          color: ink,
          fontSize: 20,
          fontWeight: FontWeight.w900,
        ),
        contentTextStyle: TextStyle(
          color: ink,
          fontSize: 14,
          fontWeight: FontWeight.w500,
        ),
      ),
      bottomSheetTheme: const BottomSheetThemeData(
        backgroundColor: paper,
        surfaceTintColor: Colors.transparent,
        elevation: 0,
        shape: RoundedRectangleBorder(
          side: BorderSide(color: ink, width: borderWidth),
          borderRadius: BorderRadius.vertical(top: Radius.circular(radius)),
        ),
      ),
      inputDecorationTheme: const InputDecorationTheme(
        filled: true,
        fillColor: surface,
        labelStyle: TextStyle(color: ink, fontWeight: FontWeight.w800),
        hintStyle: TextStyle(color: muted, fontWeight: FontWeight.w500),
        prefixIconColor: ink,
        suffixIconColor: ink,
        contentPadding: EdgeInsets.symmetric(horizontal: 14, vertical: 14),
        border: OutlineInputBorder(
          borderSide: BorderSide(color: ink, width: borderWidth),
          borderRadius: BorderRadius.all(Radius.circular(radius)),
        ),
        enabledBorder: OutlineInputBorder(
          borderSide: BorderSide(color: ink, width: borderWidth),
          borderRadius: BorderRadius.all(Radius.circular(radius)),
        ),
        focusedBorder: OutlineInputBorder(
          borderSide: BorderSide(color: pink, width: 4),
          borderRadius: BorderRadius.all(Radius.circular(radius)),
        ),
        errorBorder: OutlineInputBorder(
          borderSide: BorderSide(color: danger, width: 4),
          borderRadius: BorderRadius.all(Radius.circular(radius)),
        ),
      ),
      elevatedButtonTheme: ElevatedButtonThemeData(
        style: buttonStyle.copyWith(
          backgroundColor: const WidgetStatePropertyAll(yellow),
        ),
      ),
      filledButtonTheme: FilledButtonThemeData(
        style: buttonStyle.copyWith(
          backgroundColor: const WidgetStatePropertyAll(cyan),
        ),
      ),
      outlinedButtonTheme: OutlinedButtonThemeData(
        style: buttonStyle.copyWith(
          backgroundColor: const WidgetStatePropertyAll(surface),
        ),
      ),
      textButtonTheme: TextButtonThemeData(
        style: buttonStyle.copyWith(
          backgroundColor: const WidgetStatePropertyAll(surface),
          side: const WidgetStatePropertyAll(
            BorderSide(color: ink, width: compactBorderWidth),
          ),
          shape: const WidgetStatePropertyAll(compactShape),
          minimumSize: const WidgetStatePropertyAll(Size(44, 44)),
          padding: const WidgetStatePropertyAll(
            EdgeInsets.symmetric(horizontal: 12, vertical: 8),
          ),
        ),
      ),
      iconButtonTheme: const IconButtonThemeData(
        style: ButtonStyle(
          minimumSize: WidgetStatePropertyAll(Size(44, 44)),
          foregroundColor: WidgetStatePropertyAll(ink),
          backgroundColor: WidgetStatePropertyAll(surface),
          side: WidgetStatePropertyAll(
            BorderSide(color: ink, width: compactBorderWidth),
          ),
          shape: WidgetStatePropertyAll(compactShape),
        ),
      ),
      listTileTheme: const ListTileThemeData(
        textColor: ink,
        iconColor: ink,
        selectedColor: ink,
        selectedTileColor: cyan,
        contentPadding: EdgeInsets.symmetric(horizontal: 12, vertical: 4),
        shape: compactShape,
      ),
      dividerTheme: const DividerThemeData(
        color: ink,
        thickness: compactBorderWidth,
        space: compactBorderWidth,
      ),
      chipTheme: const ChipThemeData(
        backgroundColor: cyan,
        selectedColor: yellow,
        disabledColor: Color(0xFFE4E0D7),
        side: BorderSide(color: ink, width: compactBorderWidth),
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.all(Radius.circular(4)),
        ),
        labelStyle: TextStyle(color: ink, fontWeight: FontWeight.w800),
      ),
      segmentedButtonTheme: const SegmentedButtonThemeData(
        style: ButtonStyle(
          foregroundColor: WidgetStatePropertyAll(ink),
          backgroundColor: WidgetStatePropertyAll(surface),
          side: WidgetStatePropertyAll(
            BorderSide(color: ink, width: compactBorderWidth),
          ),
          shape: WidgetStatePropertyAll(compactShape),
          textStyle: WidgetStatePropertyAll(
            TextStyle(fontWeight: FontWeight.w800),
          ),
        ),
      ),
      snackBarTheme: const SnackBarThemeData(
        backgroundColor: ink,
        contentTextStyle: TextStyle(color: paper, fontWeight: FontWeight.w800),
        actionTextColor: yellow,
        behavior: SnackBarBehavior.floating,
        elevation: 0,
        shape: RoundedRectangleBorder(
          side: BorderSide(color: paper, width: compactBorderWidth),
          borderRadius: BorderRadius.all(Radius.circular(4)),
        ),
      ),
      progressIndicatorTheme: const ProgressIndicatorThemeData(
        color: pink,
        linearTrackColor: surface,
        circularTrackColor: surface,
        linearMinHeight: 10,
      ),
      checkboxTheme: CheckboxThemeData(
        fillColor: WidgetStateProperty.resolveWith(
          (states) => states.contains(WidgetState.selected) ? yellow : surface,
        ),
        checkColor: const WidgetStatePropertyAll(ink),
        side: const BorderSide(color: ink, width: compactBorderWidth),
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(2)),
      ),
      switchTheme: SwitchThemeData(
        thumbColor: const WidgetStatePropertyAll(ink),
        trackColor: WidgetStateProperty.resolveWith(
          (states) => states.contains(WidgetState.selected) ? mint : surface,
        ),
        trackOutlineColor: const WidgetStatePropertyAll(ink),
        trackOutlineWidth: const WidgetStatePropertyAll(compactBorderWidth),
      ),
      tabBarTheme: const TabBarThemeData(
        labelColor: ink,
        unselectedLabelColor: muted,
        indicatorColor: pink,
        indicatorSize: TabBarIndicatorSize.tab,
        dividerColor: ink,
        labelStyle: TextStyle(fontWeight: FontWeight.w900),
      ),
      tooltipTheme: const TooltipThemeData(
        decoration: BoxDecoration(
          color: ink,
          borderRadius: BorderRadius.all(Radius.circular(3)),
        ),
        textStyle: TextStyle(color: paper, fontWeight: FontWeight.w700),
      ),
    );
  }
}

/// A reusable hard-shadow surface for high-priority sections.
class NeobrutalPanel extends StatelessWidget {
  const NeobrutalPanel({
    super.key,
    required this.child,
    this.color = NeobrutalTheme.surface,
    this.padding = const EdgeInsets.all(16),
    this.margin = EdgeInsets.zero,
    this.shadow = true,
    this.borderColor = NeobrutalTheme.ink,
  });

  final Widget child;
  final Color color;
  final EdgeInsetsGeometry padding;
  final EdgeInsetsGeometry margin;
  final bool shadow;
  final Color borderColor;

  @override
  Widget build(BuildContext context) => Container(
    margin: margin,
    padding: padding,
    decoration: NeobrutalTheme.panel(
      color: color,
      borderColor: borderColor,
      shadow: shadow,
    ),
    child: child,
  );
}

/// Tactile Neobrutalist interactive container wrapper.
///
/// Provides:
/// - Crisp ink border & hard offset shadow (default Offset(3, 3))
/// - Animated tactile press micro-interaction (70ms Curves.easeOutQuad):
///   - On pointer down: translates (+2, +2) and collapses shadow to Offset(0, 0)
///   - Trigger selection haptic feedback
///   - On pointer up/cancel: springs back to resting position
class NeobrutalPressable extends StatefulWidget {
  final Widget child;
  final VoidCallback? onTap;
  final Offset shadowOffset;
  final Color shadowColor;
  final double cornerRadius;
  final bool enabled;

  const NeobrutalPressable({
    super.key,
    required this.child,
    this.onTap,
    this.shadowOffset = const Offset(3, 3),
    this.shadowColor = NeobrutalTheme.ink,
    this.cornerRadius = 6,
    this.enabled = true,
  });

  @override
  State<NeobrutalPressable> createState() => _NeobrutalPressableState();
}

class _NeobrutalPressableState extends State<NeobrutalPressable> {
  bool _isPressed = false;

  void _onPointerDown(PointerDownEvent _) {
    if (!widget.enabled) return;
    setState(() => _isPressed = true);
    HapticFeedback.selectionClick();
  }

  void _onPointerUp(PointerUpEvent _) {
    if (!widget.enabled) return;
    if (_isPressed) setState(() => _isPressed = false);
  }

  void _onPointerCancel(PointerCancelEvent _) {
    if (!widget.enabled) return;
    if (_isPressed) setState(() => _isPressed = false);
  }

  @override
  Widget build(BuildContext context) {
    final currentShadow = _isPressed ? Offset.zero : widget.shadowOffset;
    final translate = _isPressed ? const Offset(2, 2) : Offset.zero;

    Widget body = AnimatedContainer(
      duration: const Duration(milliseconds: 70),
      curve: Curves.easeOutQuad,
      transform: Matrix4.translationValues(translate.dx, translate.dy, 0),
      decoration: BoxDecoration(
        borderRadius: BorderRadius.circular(widget.cornerRadius),
        boxShadow: [
          BoxShadow(
            color: widget.shadowColor,
            offset: currentShadow,
            blurRadius: 0,
          ),
        ],
      ),
      child: widget.child,
    );

    if (widget.onTap != null) {
      body = GestureDetector(
        onTap: widget.enabled ? widget.onTap : null,
        behavior: HitTestBehavior.opaque,
        child: body,
      );
    }

    return Listener(
      onPointerDown: _onPointerDown,
      onPointerUp: _onPointerUp,
      onPointerCancel: _onPointerCancel,
      child: body,
    );
  }
}

/// Static flat badge / tag with crisp ink outline and ZERO shadow.
///
/// Used for non-interactive status, tags, counts, and descriptions.
class NeobrutalBadge extends StatelessWidget {
  final Widget child;
  final Color bg;
  final Color borderColor;
  final double borderWidth;
  final double cornerRadius;
  final EdgeInsetsGeometry padding;

  const NeobrutalBadge({
    super.key,
    required this.child,
    this.bg = NeobrutalTheme.paper,
    this.borderColor = NeobrutalTheme.ink,
    this.borderWidth = NeobrutalTheme.compactBorderWidth,
    this.cornerRadius = 4,
    this.padding = const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
  });

  @override
  Widget build(BuildContext context) => Container(
    padding: padding,
    decoration: BoxDecoration(
      color: bg,
      borderRadius: BorderRadius.circular(cornerRadius),
      border: Border.all(color: borderColor, width: borderWidth),
    ),
    child: child,
  );
}

/// Smooth staggered entrance animation (fade + slide-up) for view sections.
class StaggeredCardEntry extends StatelessWidget {
  final Animation<double> animation;
  final double startInterval;
  final double endInterval;
  final Widget child;

  const StaggeredCardEntry({
    super.key,
    required this.animation,
    required this.startInterval,
    required this.endInterval,
    required this.child,
  });

  @override
  Widget build(BuildContext context) {
    final curved = CurvedAnimation(
      parent: animation,
      curve: Interval(startInterval, endInterval, curve: Curves.easeOutCubic),
    );

    return SlideTransition(
      position: Tween<Offset>(
        begin: const Offset(0, 0.08),
        end: Offset.zero,
      ).animate(curved),
      child: FadeTransition(
        opacity: Tween<double>(begin: 0.0, end: 1.0).animate(curved),
        child: child,
      ),
    );
  }
}
