import 'dart:convert';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:vrv_desk/src/models/device.dart';
import 'package:vrv_desk/src/views/home_view.dart';
import 'package:vrv_desk/src/views/mirror_view.dart';
import 'package:vrv_desk/src/widgets/pin_dialog.dart';
import 'test_websocket_helper.dart';

void main() {
  group('MirrorView Auth Handshake Tests', () {
    late MockWebSocket mockSocket;

    setUp(() {
      mockSocket = MockWebSocket();
    });

    tearDown(() async {
      // await mockSocket.close();
    });

    testWidgets('MirrorView displays PIN dialog / overlay when initialPin is null and receives auth_required', (tester) async {
      await tester.pumpWidget(
        MaterialApp(
          home: MirrorView(
            hostIp: '192.168.1.100',
            port: 53211,
            initialPin: null,
            webSocketConnector: (_) => Future.value(mockSocket),
          ),
        ),
      );

      // Connected state
      await tester.pump();
      expect(find.text('Authentication Required'), findsOneWidget);
      expect(find.text('AUTH REQUIRED'), findsOneWidget);

      // Host sends auth_required
      mockSocket.feedIncoming(jsonEncode({
        'type': 'auth_required',
        'host_name': 'Host-PC',
        'version': '0.4.0',
      }));

      await tester.pump();
      await tester.pump(const Duration(milliseconds: 100));

      // Verify that PinDialog is displayed with host info
      expect(find.byType(PinDialog), findsOneWidget);
      expect(find.text('Connect to Host PC (192.168.1.100)'), findsOneWidget);
      expect(find.byKey(const Key('pin_input_field')), findsOneWidget);

      // Verify background overlay also displays auth required status
      expect(find.text('Authentication Required'), findsOneWidget);

      // Listen for client's auth_verify response
      final sentMessages = <Map<String, dynamic>>[];
      mockSocket.outgoing.listen((data) {
        if (data is String) {
          sentMessages.add(jsonDecode(data) as Map<String, dynamic>);
        }
      });

      // User enters PIN in the dialog
      await tester.enterText(find.byKey(const Key('pin_input_field')), '654321');
      await tester.pump();
      await tester.tap(find.byKey(const Key('pin_submit_button')));
      await tester.pump();

      expect(sentMessages.length, equals(1));
      expect(sentMessages.first['type'], equals('auth_verify'));
      expect(sentMessages.first['pin'], equals('654321'));

      // Host sends auth_ok
      mockSocket.feedIncoming(jsonEncode({
        'type': 'auth_ok',
        'session_token': 'abc_token_123',
      }));

      await tester.pump();
      await tester.pump(const Duration(milliseconds: 100));

      // Verify authenticated UI state
      expect(find.text('LIVE (0f)'), findsOneWidget);
      expect(find.text('Connected & Authenticated with Host PC'), findsOneWidget);
      expect(find.byType(PinDialog), findsNothing);
    });

    testWidgets('MirrorView automatically sends auth_verify when initialPin is supplied', (tester) async {
      final sentMessages = <Map<String, dynamic>>[];
      mockSocket.outgoing.listen((data) {
        if (data is String) {
          sentMessages.add(jsonDecode(data) as Map<String, dynamic>);
        }
      });

      await tester.pumpWidget(
        MaterialApp(
          home: MirrorView(
            hostIp: '192.168.1.50',
            port: 53211,
            initialPin: '849201',
            webSocketConnector: (_) => Future.value(mockSocket),
          ),
        ),
      );

      await tester.pump();

      // Host sends auth_required
      mockSocket.feedIncoming(jsonEncode({
        'type': 'auth_required',
        'host_name': 'Host-PC',
        'version': '0.4.0',
      }));

      await tester.pump();

      // Client should automatically respond with auth_verify
      expect(sentMessages.length, equals(1));
      expect(sentMessages.first['type'], equals('auth_verify'));
      expect(sentMessages.first['pin'], equals('849201'));

      // PinDialog should NOT have been shown
      expect(find.byType(PinDialog), findsNothing);

      // Now host responds with auth_ok
      mockSocket.feedIncoming(jsonEncode({
        'type': 'auth_ok',
        'session_token': 'random_token_xyz',
      }));
      await tester.pump();

      expect(find.text('LIVE (0f)'), findsOneWidget);
      expect(find.text('Connected & Authenticated with Host PC'), findsOneWidget);
    });

    testWidgets('MirrorView handles auth_failed and allows retry with remaining attempts', (tester) async {
      final sentMessages = <Map<String, dynamic>>[];
      mockSocket.outgoing.listen((data) {
        if (data is String) {
          sentMessages.add(jsonDecode(data) as Map<String, dynamic>);
        }
      });

      await tester.pumpWidget(
        MaterialApp(
          home: MirrorView(
            hostIp: '192.168.1.50',
            port: 53211,
            initialPin: '000000',
            webSocketConnector: (_) => Future.value(mockSocket),
          ),
        ),
      );

      await tester.pump();

      // Host sends auth_required -> MirrorView automatically sends '000000'
      mockSocket.feedIncoming(jsonEncode({'type': 'auth_required'}));
      await tester.pump();
      expect(sentMessages.length, equals(1));
      expect(sentMessages.first['pin'], equals('000000'));

      // Host responds with auth_failed (2 attempts left)
      mockSocket.feedIncoming(jsonEncode({
        'type': 'auth_failed',
        'reason': 'Invalid PIN',
        'remaining_attempts': 2,
      }));
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 100));

      // Error message and dialog shown
      expect(find.textContaining('Authentication failed: Invalid PIN (2 attempts remaining)'), findsOneWidget);
      expect(find.text('Authentication Required (2 attempts left)'), findsOneWidget);
      expect(find.byType(PinDialog), findsOneWidget);

      // User retries with correct PIN
      await tester.enterText(find.byKey(const Key('pin_input_field')), '654321');
      await tester.pump();
      await tester.tap(find.byKey(const Key('pin_submit_button')));
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 100));

      expect(sentMessages.length, equals(2));
      expect(sentMessages[1]['pin'], equals('654321'));

      // Host accepts PIN
      mockSocket.feedIncoming(jsonEncode({
        'type': 'auth_ok',
        'session_token': 'valid_token',
      }));
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 100));

      expect(find.text('LIVE (0f)'), findsOneWidget);
      expect(find.byType(PinDialog), findsNothing);
    });

    testWidgets('MirrorView locks out and shows fatal error on 0 remaining attempts', (tester) async {
      await tester.pumpWidget(
        MaterialApp(
          home: MirrorView(
            hostIp: '192.168.1.50',
            port: 53211,
            initialPin: '000000',
            webSocketConnector: (_) => Future.value(mockSocket),
          ),
        ),
      );

      await tester.pump();

      // Host sends auth_required
      mockSocket.feedIncoming(jsonEncode({'type': 'auth_required'}));
      await tester.pump();

      // Host sends fatal auth_failed (0 attempts remaining)
      mockSocket.feedIncoming(jsonEncode({
        'type': 'auth_failed',
        'reason': 'Too many attempts',
        'remaining_attempts': 0,
      }));
      await tester.pump();

      expect(find.textContaining('Authentication failed: Too many attempts. Connection locked.'), findsOneWidget);
      expect(find.text('Connection locked due to failed attempts.'), findsOneWidget);
      expect(find.byKey(const Key('enter_pin_overlay_button')), findsNothing);

      // Advance clock past pop timer
      await tester.pump(const Duration(seconds: 3));
    });
  });

  group('HomeView PIN Handshake Integration', () {
    testWidgets('passes initialPin from direct connect dialog to MirrorView', (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: HomeView(
            myDeviceId: '123456',
            initialDevices: [],
          ),
        ),
      );

      await tester.pump();

      // Tap direct connect action icon in AppBar
      await tester.tap(find.byKey(const Key('direct_connect_button')));
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 300));

      expect(find.text('Connect to PC / Host'), findsOneWidget);
      expect(find.byKey(const Key('direct_pin_field')), findsOneWidget);

      // Enter IP and PIN
      await tester.enterText(find.byKey(const Key('direct_ip_field')), '192.168.1.99');
      await tester.enterText(find.byKey(const Key('direct_pin_field')), '789012');
      await tester.pump();

      // Tap Connect button
      await tester.tap(find.byKey(const Key('dialog_connect_button')));
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 300));

      // Verify MirrorView is pushed with initialPin
      final mirrorViewFinder = find.byType(MirrorView);
      expect(mirrorViewFinder, findsOneWidget);
      final MirrorView mirrorView = tester.widget(mirrorViewFinder);
      expect(mirrorView.hostIp, equals('192.168.1.99'));
      expect(mirrorView.initialPin, equals('789012'));

      // Fast forward past connect timeout
      await tester.pump(const Duration(seconds: 6));
    });

    testWidgets('passes initialPin from discovered device pin dialog to MirrorView', (tester) async {
      final device = DiscoveredDevice(
        deviceId: 'dev_123',
        deviceName: 'Host-PC',
        osType: 'windows',
        ipAddress: '192.168.1.200',
        port: 53211,
        lastSeen: DateTime.now(),
      );

      await tester.pumpWidget(
        MaterialApp(
          home: HomeView(
            myDeviceId: '123456',
            initialDevices: [device],
          ),
        ),
      );

      await tester.pump();

      // Tap connect on discovered device
      final connectBtnFinder = find.byKey(Key('connect_button_${device.deviceId}'));
      expect(connectBtnFinder, findsOneWidget);
      await tester.ensureVisible(connectBtnFinder);
      await tester.tap(connectBtnFinder);
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 300));

      // PinDialog is visible
      expect(find.byType(PinDialog), findsOneWidget);
      await tester.enterText(find.byKey(const Key('pin_input_field')), '998877');
      await tester.pump();

      await tester.tap(find.byKey(const Key('pin_submit_button')));
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 300));

      final mirrorViewFinder = find.byType(MirrorView);
      expect(mirrorViewFinder, findsOneWidget);

      final MirrorView mirrorView = tester.widget(mirrorViewFinder);
      expect(mirrorView.hostIp, equals('192.168.1.200'));
      expect(mirrorView.port, equals(53211));
      expect(mirrorView.initialPin, equals('998877'));

      // Fast forward past connect timeout
      await tester.pump(const Duration(seconds: 6));
    });
  });
}
