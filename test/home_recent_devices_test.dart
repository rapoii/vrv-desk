import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:vrv_desk/src/models/recent_device.dart';
import 'package:vrv_desk/src/services/recent_devices_storage.dart';
import 'package:vrv_desk/src/views/home_view.dart';
import 'package:vrv_desk/src/views/mirror_view.dart';

void main() {
  setUp(() {
    RecentDevicesStorage.clear();
  });

  tearDown(() {
    RecentDevicesStorage.clear();
  });

  group('HomeView Recent Devices Widget Tests', () {
    testWidgets('Renders Recent Devices section when devices exist and handles 1-tap reconnect', (tester) async {
      RecentDevicesStorage.addOrUpdate(
        RecentDevice(
          deviceId: '849201',
          name: 'LAPTOP-PONGO',
          osType: 'pc',
          lastConnected: DateTime.now().subtract(const Duration(minutes: 5)),
          savedPin: '123456',
        ),
      );

      await tester.pumpWidget(
        const MaterialApp(
          home: HomeView(myDeviceId: '112233'),
        ),
      );
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 400));

      // Recent devices header
      expect(find.text('Recent Devices'), findsOneWidget);
      expect(find.text('LAPTOP-PONGO'), findsOneWidget);
      expect(find.textContaining('ID: 849201'), findsOneWidget);

      // Tap reconnect
      final reconnectBtnFinder = find.byKey(const Key('reconnect_button_849201'));
      expect(reconnectBtnFinder, findsOneWidget);
      await tester.tap(reconnectBtnFinder);
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 300));

      // MirrorView opens
      expect(find.byType(MirrorView), findsOneWidget);

      // Fast forward past connection timeout
      await tester.pump(const Duration(seconds: 6));
    });

    testWidgets('Clear button removes all recent devices and hides section', (tester) async {
      RecentDevicesStorage.addOrUpdate(
        RecentDevice(
          deviceId: '998877',
          name: 'Office-Rig',
          osType: 'pc',
          lastConnected: DateTime.now(),
        ),
      );

      await tester.pumpWidget(
        const MaterialApp(
          home: HomeView(myDeviceId: '112233'),
        ),
      );
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 400));

      expect(find.text('Recent Devices'), findsOneWidget);
      expect(find.text('Office-Rig'), findsOneWidget);

      // Tap Clear
      final clearBtnFinder = find.byKey(const Key('clear_recent_button'));
      expect(clearBtnFinder, findsOneWidget);
      await tester.tap(clearBtnFinder);
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 300));

      // Recent Devices section hidden
      expect(find.text('Recent Devices'), findsNothing);
      expect(find.text('Office-Rig'), findsNothing);
    });
  });
}