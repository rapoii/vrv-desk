import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:vrv_desk/src/models/device.dart';
import 'package:vrv_desk/src/views/home_view.dart';
import 'package:vrv_desk/src/widgets/pin_dialog.dart';

void main() {
  group('DiscoveredDevice Model', () {
    test('creates DiscoveredDevice instance and equality works', () {
      final now = DateTime.now();
      final dev1 = DiscoveredDevice(
        deviceId: '123456',
        deviceName: 'Desktop-PC',
        osType: 'windows',
        ipAddress: '192.168.1.10',
        port: 8080,
        lastSeen: now,
      );

      final dev2 = DiscoveredDevice(
        deviceId: '123456',
        deviceName: 'Desktop-PC',
        osType: 'windows',
        ipAddress: '192.168.1.10',
        port: 8080,
        lastSeen: now,
      );

      expect(dev1, equals(dev2));
      expect(dev1.hashCode, equals(dev2.hashCode));
      expect(dev1.deviceId, '123456');
      expect(dev1.deviceName, 'Desktop-PC');
      expect(dev1.osType, 'windows');
      expect(dev1.ipAddress, '192.168.1.10');
      expect(dev1.port, 8080);
    });
  });

  group('PinDialog Widget', () {
    testWidgets('renders device name and validates 6-digit pin input', (tester) async {
      String? submittedPin;

      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: PinDialog(
              deviceName: 'Test-Device',
              onSubmitted: (pin) {
                submittedPin = pin;
              },
            ),
          ),
        ),
      );

      expect(find.text('Connect to Test-Device'), findsOneWidget);
      expect(find.byKey(const Key('pin_input_field')), findsOneWidget);

      final submitBtnFinder = find.byKey(const Key('pin_submit_button'));
      expect(submitBtnFinder, findsOneWidget);

      // Initially disabled since input is empty
      final ElevatedButton submitBtn = tester.widget(submitBtnFinder);
      expect(submitBtn.enabled, isFalse);

      // Enter 4 digits (invalid)
      await tester.enterText(find.byKey(const Key('pin_input_field')), '1234');
      await tester.pump();
      final ElevatedButton submitBtn2 = tester.widget(submitBtnFinder);
      expect(submitBtn2.enabled, isFalse);

      // Enter 6 non-digit characters (invalid)
      await tester.enterText(find.byKey(const Key('pin_input_field')), 'abcdef');
      await tester.pump();
      final ElevatedButton submitBtn3 = tester.widget(submitBtnFinder);
      expect(submitBtn3.enabled, isFalse);

      // Enter 6 valid digits
      await tester.enterText(find.byKey(const Key('pin_input_field')), '654321');
      await tester.pump();
      final ElevatedButton submitBtn4 = tester.widget(submitBtnFinder);
      expect(submitBtn4.enabled, isTrue);

      // Tap submit button
      await tester.tap(submitBtnFinder);
      await tester.pumpAndSettle();

      expect(submittedPin, equals('654321'));
    });
  });

  group('HomeView Widget', () {
    testWidgets('displays device ID and scanning state when device list is empty', (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: HomeView(
            myDeviceId: '889900',
            initialDevices: [],
          ),
        ),
      );

      expect(find.text('889900'), findsOneWidget);
      expect(find.text('0 found'), findsOneWidget);
      expect(find.text('Scanning for devices on local network...'), findsOneWidget);
      expect(find.byType(CircularProgressIndicator), findsOneWidget);
    });

    testWidgets('displays discovered devices list and triggers pin dialog on Connect click', (tester) async {
      final devices = [
        DiscoveredDevice(
          deviceId: '111222',
          deviceName: 'Windows-Host',
          osType: 'windows',
          ipAddress: '192.168.1.50',
          port: 50000,
          lastSeen: DateTime.now(),
        ),
        DiscoveredDevice(
          deviceId: '333444',
          deviceName: 'Android-Client',
          osType: 'android',
          ipAddress: '192.168.1.51',
          port: 50000,
          lastSeen: DateTime.now(),
        ),
      ];

      DiscoveredDevice? connectedDevice;
      String? connectedPin;

      await tester.pumpWidget(
        MaterialApp(
          home: HomeView(
            myDeviceId: '999888',
            initialDevices: devices,
            onConnect: (device, pin) {
              connectedDevice = device;
              connectedPin = pin;
            },
          ),
        ),
      );

      expect(find.text('999888'), findsOneWidget);
      expect(find.text('2 found'), findsOneWidget);
      expect(find.text('Windows-Host'), findsOneWidget);
      expect(find.text('Android-Client'), findsOneWidget);

      // Tap Connect on first device
      final connectBtnFinder = find.byKey(const Key('connect_button_111222'));
      expect(connectBtnFinder, findsOneWidget);
      await tester.tap(connectBtnFinder);
      await tester.pumpAndSettle();

      // Verify PIN dialog opened
      expect(find.text('Connect to Windows-Host'), findsOneWidget);

      // Enter PIN and submit
      await tester.enterText(find.byKey(const Key('pin_input_field')), '123456');
      await tester.pump();
      await tester.tap(find.byKey(const Key('pin_submit_button')));
      await tester.pumpAndSettle();

      expect(connectedDevice, equals(devices[0]));
      expect(connectedPin, equals('123456'));
    });

    testWidgets('copy device ID button works and shows SnackBar', (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: HomeView(
            myDeviceId: '123456',
            initialDevices: [],
          ),
        ),
      );

      final copyBtnFinder = find.byKey(const Key('copy_device_id_button'));
      expect(copyBtnFinder, findsOneWidget);

      await tester.tap(copyBtnFinder);
      await tester.pump(); // trigger SnackBar animation
      await tester.pump(const Duration(milliseconds: 100));

      expect(find.text('Device ID copied to clipboard'), findsOneWidget);
      expect(find.text('Copied'), findsOneWidget);
    });
  });
}
