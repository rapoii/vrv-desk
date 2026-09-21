import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:vrv_desk/src/widgets/shortcut_bar.dart';

void main() {
  group('ShortcutBar Widget', () {
    testWidgets('renders all required shortcut buttons and triggers onShortcutPressed', (tester) async {
      final List<String> pressedShortcuts = [];

      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: ShortcutBar(
              onShortcutPressed: (shortcut) {
                pressedShortcuts.add(shortcut);
              },
            ),
          ),
        ),
      );

      // Verify all buttons are displayed
      expect(find.text('Win'), findsOneWidget);
      expect(find.text('Esc'), findsOneWidget);
      expect(find.text('Enter'), findsOneWidget);
      expect(find.text('Del'), findsOneWidget);
      expect(find.text('Tab'), findsOneWidget);
      expect(find.text('TaskMgr'), findsOneWidget);
      expect(find.text('Alt+Tab'), findsOneWidget);
      expect(find.text('Desktop'), findsOneWidget);
      expect(find.text('Ctrl+Alt+Del'), findsOneWidget);
      expect(find.text('Elevate'), findsOneWidget);
      expect(find.text('Monitor'), findsOneWidget);
      expect(find.text('Privacy'), findsOneWidget);
      expect(find.text('Lock PC'), findsOneWidget);

      // Tap buttons and verify correct shortcut identifiers are passed
      await tester.tap(find.text('Win'));
      await tester.pump();
      expect(pressedShortcuts.last, 'win');

      await tester.tap(find.text('Esc'));
      await tester.pump();
      expect(pressedShortcuts.last, 'esc');

      await tester.tap(find.text('Enter'));
      await tester.pump();
      expect(pressedShortcuts.last, 'enter');

      await tester.tap(find.text('Del'));
      await tester.pump();
      expect(pressedShortcuts.last, 'backspace');

      await tester.tap(find.text('Tab'));
      await tester.pump();
      expect(pressedShortcuts.last, 'tab');

      await tester.tap(find.text('TaskMgr'));
      await tester.pump();
      expect(pressedShortcuts.last, 'task_manager');

      await tester.tap(find.text('Alt+Tab'));
      await tester.pump();
      expect(pressedShortcuts.last, 'alt_tab');

      await tester.tap(find.text('Desktop'));
      await tester.pump();
      expect(pressedShortcuts.last, 'show_desktop');

      await tester.ensureVisible(find.text('Ctrl+Alt+Del'));
      await tester.tap(find.text('Ctrl+Alt+Del'));
      await tester.pump();
      expect(pressedShortcuts.last, 'ctrl_alt_del');

      await tester.ensureVisible(find.text('Elevate'));
      await tester.tap(find.text('Elevate'));
      await tester.pump();
      expect(pressedShortcuts.last, 'elevate');

      await tester.ensureVisible(find.text('Monitor'));
      await tester.tap(find.text('Monitor'));
      await tester.pump();
      expect(pressedShortcuts.last, 'switch_monitor');

      await tester.ensureVisible(find.text('Privacy'));
      await tester.tap(find.text('Privacy'));
      await tester.pump();
      expect(pressedShortcuts.last, 'privacy_mode');

      await tester.ensureVisible(find.text('Lock PC'));
      await tester.tap(find.text('Lock PC'));
      await tester.pump();
      expect(pressedShortcuts.last, 'lock_pc');

      expect(pressedShortcuts, [
        'win',
        'esc',
        'enter',
        'backspace',
        'tab',
        'task_manager',
        'alt_tab',
        'show_desktop',
        'ctrl_alt_del',
        'elevate',
        'switch_monitor',
        'privacy_mode',
        'lock_pc',
      ]);
    });
  });
}
