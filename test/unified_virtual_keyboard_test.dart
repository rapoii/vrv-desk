import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:vrv_desk/src/widgets/unified_virtual_keyboard.dart';

void main() {
  group('UnifiedVirtualKeyboard Tests', () {
    testWidgets('Direct mode: taps immediately send text and shortcuts',
        (tester) async {
      String lastText = '';
      String lastShortcut = '';
      bool closed = false;

      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: UnifiedVirtualKeyboard(
              initialDirectMode: true,
              onTextInput: (val) => lastText = val,
              onShortcut: (sc) => lastShortcut = sc,
              onClose: () => closed = true,
            ),
          ),
        ),
      );

      // Verify dock exists
      expect(find.byKey(const Key('unified_keyboard_dock')), findsOneWidget);

      // Tap key 'A'
      await tester.tap(find.text('a'));
      await tester.pump();
      expect(lastText, 'a');

      // Tap ESC
      await tester.tap(find.text('ESC'));
      await tester.pump();
      expect(lastShortcut, 'esc');

      // Tap BKSP
      await tester.tap(find.byKey(const Key('keyboard_key_backspace')));
      await tester.pump();
      expect(lastShortcut, 'backspace');

      // Tap SPACE
      await tester.tap(find.text('SPACE'));
      await tester.pump();
      expect(lastShortcut, 'space');

      // Close button
      await tester.tap(find.byKey(const Key('keyboard_close_button')));
      await tester.pump();
      expect(closed, isTrue);
    });

    testWidgets('Sticky Ctrl combo: Ctrl + C sends ctrl_c shortcut',
        (tester) async {
      String lastText = '';
      String lastShortcut = '';

      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: UnifiedVirtualKeyboard(
              initialDirectMode: true,
              onTextInput: (val) => lastText = val,
              onShortcut: (sc) => lastShortcut = sc,
              onClose: () {},
            ),
          ),
        ),
      );

      // Tap CTRL in top helper row
      await tester.tap(find.text('CTRL'));
      await tester.pump();

      // Tap 'C'
      await tester.tap(find.text('c'));
      await tester.pump();

      expect(lastShortcut, 'ctrl_c');
      expect(lastText, '');
    });

    testWidgets(
        'Buffered mode: accumulates text in box and sends on SEND tap',
        (tester) async {
      String sentText = '';
      String lastShortcut = '';

      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: UnifiedVirtualKeyboard(
              initialDirectMode: false,
              onTextInput: (val) => sentText = val,
              onShortcut: (sc) => lastShortcut = sc,
              onClose: () {},
            ),
          ),
        ),
      );

      // Preview text field exists in buffered mode
      expect(find.byKey(const Key('keyboard_buffer_text_field')), findsOneWidget);

      // Type "hello"
      await tester.tap(find.text('h'));
      await tester.pump();
      await tester.tap(find.text('e'));
      await tester.pump();
      await tester.tap(find.text('l'));
      await tester.pump();
      await tester.tap(find.text('l'));
      await tester.pump();
      await tester.tap(find.text('o'));
      await tester.pump();

      // In buffered mode, text is NOT sent yet!
      expect(sentText, '');
      expect(find.text('hello'), findsOneWidget);

      // Tap SEND button
      await tester.tap(find.byKey(const Key('keyboard_buffer_send_button')));
      await tester.pump();

      // Now text is sent!
      expect(sentText, 'hello');
      // Buffer is cleared
      expect(find.text('hello'), findsNothing);
    });

    testWidgets('Mode toggle switch: switches from Direct to Send Mode and back',
        (tester) async {
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: UnifiedVirtualKeyboard(
              initialDirectMode: true,
              onTextInput: (_) {},
              onShortcut: (_) {},
              onClose: () {},
            ),
          ),
        ),
      );

      // Initially direct mode, no preview text field
      expect(find.byKey(const Key('keyboard_buffer_text_field')), findsNothing);

      // Switch to Send Mode
      await tester.tap(find.byKey(const Key('keyboard_mode_toggle_buffered')));
      await tester.pump();

      // Text field appears
      expect(find.byKey(const Key('keyboard_buffer_text_field')), findsOneWidget);

      // Switch back to Direct
      await tester.tap(find.byKey(const Key('keyboard_mode_toggle_direct')));
      await tester.pump();

      // Text field disappears
      expect(find.byKey(const Key('keyboard_buffer_text_field')), findsNothing);
    });

    testWidgets('Layer toggle: switches to PC Keys (F1-F12, Nav) and back',
        (tester) async {
      String lastShortcut = '';

      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: UnifiedVirtualKeyboard(
              initialDirectMode: true,
              onTextInput: (_) {},
              onShortcut: (sc) => lastShortcut = sc,
              onClose: () {},
            ),
          ),
        ),
      );

      // Initially QWERTY is visible
      expect(find.text('Q'), findsNothing);
      expect(find.text('q'), findsOneWidget);
      expect(find.text('F5'), findsNothing);

      // Switch to PC Keys
      await tester.tap(find.byKey(const Key('keyboard_layer_toggle_fn')));
      await tester.pump();

      // F5, F12, HOME, END, Ctrl+Alt+Del are visible
      expect(find.text('F5'), findsOneWidget);
      expect(find.text('HOME'), findsOneWidget);
      expect(find.text('Ctrl+Alt+Del'), findsOneWidget);

      // Tap F5
      await tester.tap(find.text('F5'));
      await tester.pump();
      expect(lastShortcut, 'f5');

      // Tap Home
      await tester.tap(find.text('HOME'));
      await tester.pump();
      expect(lastShortcut, 'home');

      // Switch back to QWERTY
      await tester.tap(find.text('QWERTY Layout'));
      await tester.pump();

      expect(find.text('q'), findsOneWidget);
      expect(find.text('F5'), findsNothing);
    });
  });
}
