import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:vrv_desk/src/views/mirror_view.dart';
import 'package:vrv_desk/src/widgets/shortcut_bar.dart';

void main() {
  group('MirrorView Controls & Toolbar', () {
    testWidgets('renders floating dock, toggles shortcut bar, and finds keyboard trigger', (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: MirrorView(hostIp: '127.0.0.1', port: 53211),
        ),
      );

      // Initially ShortcutBar is not visible
      expect(find.byType(ShortcutBar), findsNothing);

      // Check floating dock buttons exist
      expect(find.byIcon(Icons.keyboard), findsOneWidget);
      expect(find.byIcon(Icons.grid_view), findsOneWidget);

      // Tap shortcut bar toggle button
      await tester.tap(find.byIcon(Icons.grid_view));
      await tester.pump();

      // ShortcutBar is now visible
      expect(find.byType(ShortcutBar), findsOneWidget);
      expect(find.text('Win'), findsOneWidget);
      expect(find.text('Esc'), findsOneWidget);

      // Tap shortcut bar toggle button again
      await tester.tap(find.byIcon(Icons.grid_view));
      await tester.pump();

      // ShortcutBar is hidden again
      expect(find.byType(ShortcutBar), findsNothing);

      // Tap keyboard toggle button
      await tester.tap(find.byIcon(Icons.keyboard));
      await tester.pump();

      // Hidden TextField exists
      expect(find.byType(TextField), findsOneWidget);

      // Fast forward past the 5-second WebSocket connect timeout
      await tester.pump(const Duration(seconds: 6));
    });

    testWidgets('expands and collapses diagnostic HUD on status pill tap', (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: MirrorView(hostIp: '127.0.0.1', port: 53211),
        ),
      );

      // Status pill exists
      final pillFinder = find.byKey(const Key('status_diagnostic_pill'));
      expect(pillFinder, findsOneWidget);

      // Initially full diagnostic details are collapsed
      expect(find.byKey(const Key('diagnostic_hud_details')), findsNothing);

      // Tap status pill to expand
      await tester.tap(pillFinder);
      await tester.pump();

      // Diagnostic HUD details are now visible
      expect(find.byKey(const Key('diagnostic_hud_details')), findsOneWidget);
      expect(find.textContaining('FPS:'), findsOneWidget);
      expect(find.textContaining('Bitrate:'), findsOneWidget);
      expect(find.textContaining('Latency:'), findsOneWidget);
      expect(find.textContaining('Loss:'), findsOneWidget);

      // Tap again to collapse
      await tester.tap(pillFinder);
      await tester.pump();
      expect(find.byKey(const Key('diagnostic_hud_details')), findsNothing);

      await tester.pump(const Duration(seconds: 6));
    });

    testWidgets('toggles input mode between Direct Touch and Trackpad mode', (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: MirrorView(hostIp: '127.0.0.1', port: 53211),
        ),
      );

      final inputToggle = find.byKey(const Key('input_mode_toggle_button'));
      expect(inputToggle, findsOneWidget);

      // Initially in Direct Touch mode, virtual mouse buttons not shown
      expect(find.byKey(const Key('trackpad_mouse_buttons')), findsNothing);

      // Tap to switch to Trackpad mode
      await tester.tap(inputToggle);
      await tester.pump();

      // Trackpad mouse buttons are visible
      expect(find.byKey(const Key('trackpad_mouse_buttons')), findsOneWidget);
      expect(find.byKey(const Key('trackpad_left_click_button')), findsOneWidget);
      expect(find.byKey(const Key('trackpad_right_click_button')), findsOneWidget);

      // Tap to switch back to Direct Touch
      await tester.tap(inputToggle);
      await tester.pump();
      expect(find.byKey(const Key('trackpad_mouse_buttons')), findsNothing);

      await tester.pump(const Duration(seconds: 6));
    });

    testWidgets('shows Quality Switcher sheet and changes profile', (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: MirrorView(hostIp: '127.0.0.1', port: 53211),
        ),
      );

      final qualityBtn = find.byKey(const Key('quality_switcher_button'));
      expect(qualityBtn, findsOneWidget);

      // Tap quality button
      await tester.tap(qualityBtn);
      await tester.pumpAndSettle();

      // Quality options visible
      expect(find.textContaining('Eco (720p 30fps)'), findsOneWidget);
      expect(find.textContaining('Balanced (1080p 60fps)'), findsOneWidget);
      expect(find.textContaining('Ultra (1080p 60fps High Bitrate)'), findsOneWidget);

      // Select Eco
      await tester.tap(find.textContaining('Eco (720p 30fps)'));
      await tester.pumpAndSettle();

      await tester.pump(const Duration(seconds: 6));
    });

    testWidgets('shows confirmation dialog on end session button tap', (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: MirrorView(hostIp: '127.0.0.1', port: 53211),
        ),
      );

      final endSessionBtn = find.byKey(const Key('end_session_button'));
      expect(endSessionBtn, findsOneWidget);

      // Tap End Session button
      await tester.tap(endSessionBtn);
      await tester.pumpAndSettle();

      // Confirmation dialog appears
      expect(find.byKey(const Key('end_session_confirm_dialog')), findsOneWidget);
      expect(find.text('End Remote Session?'), findsOneWidget);

      // Tap Cancel
      await tester.tap(find.text('Cancel'));
      await tester.pumpAndSettle();

      // Dialog dismissed, still on mirror view
      expect(find.byKey(const Key('end_session_confirm_dialog')), findsNothing);
      expect(find.byKey(const Key('end_session_button')), findsOneWidget);

      await tester.pump(const Duration(seconds: 6));
    });
  });
}
