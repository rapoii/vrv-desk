import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mirror_app/src/views/mirror_view.dart';
import 'package:mirror_app/src/widgets/shortcut_bar.dart';

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
  });
}
