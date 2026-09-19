import 'dart:convert';
import 'dart:typed_data';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:vrv_desk/src/models/device.dart';
import 'package:vrv_desk/src/services/lan_discovery_service.dart';
import 'package:vrv_desk/src/views/home_view.dart';

void main() {
  group('LanDiscoveryService Unit Tests', () {
    late LanDiscoveryService service;

    setUp(() {
      service = LanDiscoveryService();
    });

    tearDown(() async {
      await service.stop();
    });

    test('parses valid beacon JSON and emits DiscoveredDevice via stream', () async {
      final emittedDevicesList = <List<DiscoveredDevice>>[];
      final sub = service.devicesStream.listen((devices) {
        emittedDevicesList.add(devices);
      });

      final beaconMap = {
        'device_id': '849201',
        'device_name': 'My-Gaming-PC',
        'os_type': 'windows',
        'port': 53211,
        'protocol_version': 1,
      };
      final beaconBytes = Uint8List.fromList(utf8.encode(jsonEncode(beaconMap)));

      service.handleBeaconData(beaconBytes, '192.168.1.105');

      // Allow stream microtasks to process
      await Future.delayed(Duration.zero);

      expect(emittedDevicesList.length, 1);
      final devices = emittedDevicesList.first;
      expect(devices.length, 1);

      final device = devices.first;
      expect(device.deviceId, '849201');
      expect(device.deviceName, 'My-Gaming-PC');
      expect(device.osType, 'windows');
      expect(device.ipAddress, '192.168.1.105');
      expect(device.port, 53211);
      expect(service.currentDevices.length, 1);

      await sub.cancel();
    });

    test('ignores malformed or incomplete beacon payloads', () async {
      final emittedDevicesList = <List<DiscoveredDevice>>[];
      final sub = service.devicesStream.listen((devices) {
        emittedDevicesList.add(devices);
      });

      // 1. Not valid JSON
      service.handleBeaconData(Uint8List.fromList(utf8.encode('NOT_JSON')), '192.168.1.50');
      // 2. Empty device_id
      service.handleBeaconData(
        Uint8List.fromList(utf8.encode(jsonEncode({'device_id': '', 'port': 53211}))),
        '192.168.1.51',
      );
      // 3. Array instead of object
      service.handleBeaconData(
        Uint8List.fromList(utf8.encode(jsonEncode(['foo', 'bar']))),
        '192.168.1.52',
      );

      await Future.delayed(Duration.zero);
      expect(emittedDevicesList.isEmpty, isTrue);
      expect(service.currentDevices.isEmpty, isTrue);

      await sub.cancel();
    });

    test('updates lastSeen and properties when receiving beacon from same deviceId', () async {
      final beacon1 = {
        'device_id': '555666',
        'device_name': 'Initial-PC',
        'os_type': 'windows',
        'port': 53211,
        'protocol_version': 1,
      };
      service.handleBeaconData(
        Uint8List.fromList(utf8.encode(jsonEncode(beacon1))),
        '192.168.1.10',
      );

      expect(service.currentDevices.first.deviceName, 'Initial-PC');
      final firstSeen = service.currentDevices.first.lastSeen;

      await Future.delayed(const Duration(milliseconds: 10));

      final beacon2 = {
        'device_id': '555666',
        'device_name': 'Renamed-PC',
        'os_type': 'windows',
        'port': 53211,
        'protocol_version': 1,
      };
      service.handleBeaconData(
        Uint8List.fromList(utf8.encode(jsonEncode(beacon2))),
        '192.168.1.10',
      );

      expect(service.currentDevices.length, 1);
      expect(service.currentDevices.first.deviceName, 'Renamed-PC');
      expect(
        service.currentDevices.first.lastSeen.isAfter(firstSeen) ||
            service.currentDevices.first.lastSeen == firstSeen,
        isTrue,
      );
    });

    test('prunes stale devices after staleTimeout', () async {
      final beacon = {
        'device_id': '777888',
        'device_name': 'LivingRoom-PC',
        'os_type': 'windows',
        'port': 53211,
      };
      service.handleBeaconData(
        Uint8List.fromList(utf8.encode(jsonEncode(beacon))),
        '192.168.1.20',
      );
      expect(service.currentDevices.length, 1);

      // Override current time to 4 seconds later (not stale yet, staleTimeout = 6s)
      final fourSecLater = DateTime.now().add(const Duration(seconds: 4));
      service.pruneStaleDevices(nowOverride: fourSecLater);
      expect(service.currentDevices.length, 1);

      // Override current time to 7 seconds later (exceeds staleTimeout)
      final sevenSecLater = DateTime.now().add(const Duration(seconds: 7));
      service.pruneStaleDevices(nowOverride: sevenSecLater);
      expect(service.currentDevices.isEmpty, isTrue);
    });
  });

  group('HomeView LAN Discovery Integration Widget Tests', () {
    testWidgets('HomeView updates dynamically when LanDiscoveryService discovers devices',
        (tester) async {
      final mockLanService = LanDiscoveryService();

      await tester.pumpWidget(
        MaterialApp(
          home: HomeView(
            myDeviceId: '123456',
            lanDiscoveryService: mockLanService,
          ),
        ),
      );

      // Initial state: 0 found, scanning indicator displayed
      expect(find.text('0 found'), findsOneWidget);
      expect(find.text('Scanning for devices on local network...'), findsOneWidget);
      expect(find.byType(CircularProgressIndicator), findsOneWidget);

      // Simulate discovery of a Windows host beacon
      final beacon1 = {
        'device_id': '987654',
        'device_name': 'Workstation-Win11',
        'os_type': 'windows',
        'port': 53211,
      };
      mockLanService.handleBeaconData(
        Uint8List.fromList(utf8.encode(jsonEncode(beacon1))),
        '192.168.1.88',
      );

      await tester.pump();

      // UI should reflect 1 found device with name, ip/port, and OS
      expect(find.text('1 found'), findsOneWidget);
      expect(find.text('Workstation-Win11'), findsOneWidget);
      expect(find.text('192.168.1.88:53211 • windows'), findsOneWidget);
      expect(find.byKey(const Key('device_tile_987654')), findsOneWidget);
      expect(find.byKey(const Key('connect_button_987654')), findsOneWidget);

      // Simulate discovery of a 2nd device
      final beacon2 = {
        'device_id': '112233',
        'device_name': 'Surface-Pro',
        'os_type': 'windows',
        'port': 53211,
      };
      mockLanService.handleBeaconData(
        Uint8List.fromList(utf8.encode(jsonEncode(beacon2))),
        '192.168.1.92',
      );

      await tester.pump();

      expect(find.text('2 found'), findsOneWidget);
      expect(find.text('Surface-Pro'), findsOneWidget);
      expect(find.text('192.168.1.92:53211 • windows'), findsOneWidget);

      // Simulate device 1 timing out / pruning
      mockLanService.pruneStaleDevices(
        nowOverride: DateTime.now().add(const Duration(seconds: 10)),
      );

      await tester.pump();

      expect(find.text('0 found'), findsOneWidget);
      expect(find.text('Scanning for devices on local network...'), findsOneWidget);

      await mockLanService.stop();
    });

    testWidgets('Tapping Connect on discovered device opens PIN dialog with device name',
        (tester) async {
      final mockLanService = LanDiscoveryService();

      DiscoveredDevice? connectedDevice;
      String? submittedPin;

      await tester.pumpWidget(
        MaterialApp(
          home: HomeView(
            myDeviceId: '123456',
            lanDiscoveryService: mockLanService,
            onConnect: (dev, pin) {
              connectedDevice = dev;
              submittedPin = pin;
            },
          ),
        ),
      );

      // Inject discovered device
      final beacon = {
        'device_id': '456789',
        'device_name': 'Office-Rig',
        'os_type': 'windows',
        'port': 53211,
      };
      mockLanService.handleBeaconData(
        Uint8List.fromList(utf8.encode(jsonEncode(beacon))),
        '192.168.1.150',
      );

      await tester.pump();

      // Tap Connect button
      final connectBtnFinder = find.byKey(const Key('connect_button_456789'));
      expect(connectBtnFinder, findsOneWidget);
      await tester.tap(connectBtnFinder);
      await tester.pumpAndSettle();

      // Verify PIN dialog is shown
      expect(find.text('Connect to Office-Rig'), findsOneWidget);

      // Enter 6-digit PIN and submit
      await tester.enterText(find.byKey(const Key('pin_input_field')), '849201');
      await tester.pump();
      await tester.tap(find.byKey(const Key('pin_submit_button')));
      await tester.pumpAndSettle();

      expect(connectedDevice?.deviceId, '456789');
      expect(connectedDevice?.deviceName, 'Office-Rig');
      expect(connectedDevice?.ipAddress, '192.168.1.150');
      expect(submittedPin, '849201');

      await mockLanService.stop();
    });
  });
}
