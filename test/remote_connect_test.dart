import 'dart:convert';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:vrv_desk/src/views/home_view.dart';
import 'package:vrv_desk/src/views/mirror_view.dart';
import 'test_websocket_helper.dart';

void main() {
  group('Remote Device ID Parsing Tests', () {
    test('is6DigitDeviceId correctly identifies 6-digit numeric IDs with and without spaces', () {
      expect(HomeView.is6DigitDeviceId('849201'), isTrue);
      expect(HomeView.is6DigitDeviceId('849 201'), isTrue);
      expect(HomeView.is6DigitDeviceId('  123 456  '), isTrue);
      expect(HomeView.is6DigitDeviceId('000000'), isTrue);
      expect(HomeView.is6DigitDeviceId('999 999'), isTrue);

      // Invalid cases (IPs, hostnames, wrong length)
      expect(HomeView.is6DigitDeviceId('10.0.2.2'), isFalse);
      expect(HomeView.is6DigitDeviceId('192.168.1.100'), isFalse);
      expect(HomeView.is6DigitDeviceId('desktop-pc'), isFalse);
      expect(HomeView.is6DigitDeviceId('12345'), isFalse);
      expect(HomeView.is6DigitDeviceId('1234567'), isFalse);
      expect(HomeView.is6DigitDeviceId('849a01'), isFalse);
    });
  });

  group('HomeView Quick Connect and Direct Dialog Routing Tests', () {
    testWidgets('Quick Connect routes 6-digit Device ID to signaling broker URL and targetDeviceId', (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: HomeView(
            myDeviceId: '112233',
            signalingUrl: 'ws://10.0.2.2:53212',
          ),
        ),
      );

      await tester.pump();

      // Verify Quick Connect Card exists
      expect(find.byKey(const Key('quick_connect_field')), findsOneWidget);
      expect(find.byKey(const Key('quick_connect_button')), findsOneWidget);

      // Enter 6-digit Device ID with space
      await tester.enterText(find.byKey(const Key('quick_connect_field')), '849 201');
      await tester.pump();

      // Tap Quick Connect button
      await tester.tap(find.byKey(const Key('quick_connect_button')));
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 300));

      // Verify MirrorView was pushed with signalingUrl and targetDeviceId
      final mirrorViewFinder = find.byType(MirrorView);
      expect(mirrorViewFinder, findsOneWidget);
      final MirrorView mirrorView = tester.widget(mirrorViewFinder);
      expect(mirrorView.targetDeviceId, equals('849201'));
      expect(mirrorView.signalingUrl, equals('ws://10.0.2.2:53212'));
      expect(mirrorView.hostIp, isNull);

      // Fast forward past connection timeout
      await tester.pump(const Duration(seconds: 6));
    });

    testWidgets('Quick Connect routes IP address directly to hostIp', (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: HomeView(
            myDeviceId: '112233',
          ),
        ),
      );

      await tester.pump();

      // Enter IP address in Quick Connect field
      await tester.enterText(find.byKey(const Key('quick_connect_field')), '192.168.1.150');
      await tester.pump();

      // Tap Quick Connect button
      await tester.tap(find.byKey(const Key('quick_connect_button')));
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 300));

      // Verify MirrorView was pushed with direct hostIp
      final mirrorViewFinder = find.byType(MirrorView);
      expect(mirrorViewFinder, findsOneWidget);
      final MirrorView mirrorView = tester.widget(mirrorViewFinder);
      expect(mirrorView.hostIp, equals('192.168.1.150'));
      expect(mirrorView.port, equals(53211));
      expect(mirrorView.targetDeviceId, isNull);

      // Fast forward past connection timeout
      await tester.pump(const Duration(seconds: 6));
    });

    testWidgets('Direct dialog routes 6-digit Device ID to signaling broker with optional PIN', (tester) async {
      await tester.pumpWidget(
        const MaterialApp(
          home: HomeView(
            myDeviceId: '112233',
            signalingUrl: 'ws://custom-signal.local:53212',
          ),
        ),
      );

      await tester.pump();

      // Tap direct connect AppBar action icon
      await tester.tap(find.byKey(const Key('direct_connect_button')));
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 300));

      // Enter 6-digit Device ID and PIN in dialog
      await tester.enterText(find.byKey(const Key('direct_ip_field')), '987654');
      await tester.enterText(find.byKey(const Key('direct_pin_field')), '555666');
      await tester.pump();

      // Tap Connect
      await tester.tap(find.byKey(const Key('dialog_connect_button')));
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 300));

      // Verify MirrorView was pushed with signaling config and initialPin
      final mirrorViewFinder = find.byType(MirrorView);
      expect(mirrorViewFinder, findsOneWidget);
      final MirrorView mirrorView = tester.widget(mirrorViewFinder);
      expect(mirrorView.targetDeviceId, equals('987654'));
      expect(mirrorView.signalingUrl, equals('ws://custom-signal.local:53212'));
      expect(mirrorView.initialPin, equals('555666'));

      // Fast forward past connection timeout
      await tester.pump(const Duration(seconds: 6));
    });
  });

  group('MirrorView Remote Signaling Connect Flow Tests', () {
    testWidgets('sends connect_request upon WebSocket connection and handles full remote handshake & frames', (tester) async {
      final mockWs = MockWebSocket();
      final outgoingMessages = <Map<String, dynamic>>[];

      mockWs.outgoing.listen((event) {
        if (event is String) {
          try {
            outgoingMessages.add(jsonDecode(event) as Map<String, dynamic>);
          } catch (_) {}
        }
      });

      await tester.pumpWidget(
        MaterialApp(
          home: MirrorView(
            signalingUrl: 'ws://10.0.2.2:53212',
            targetDeviceId: '849201',
            initialPin: '888999',
            webSocketConnector: (url) async {
              expect(url, equals('ws://10.0.2.2:53212'));
              return mockWs;
            },
          ),
        ),
      );

      // Verify connect_request message sent to broker immediately
      await tester.pump();
      expect(outgoingMessages.length, equals(1));
      expect(outgoingMessages[0]['type'], equals('connect_request'));
      expect(outgoingMessages[0]['target_id'], equals('849201'));

      // Host responds through broker bridge with auth_required
      mockWs.feedIncoming(jsonEncode({
        'type': 'auth_required',
        'host_name': 'DESKTOP-REMOTE',
        'version': '0.5.0',
      }));
      await tester.pump();

      // Should automatically send auth_verify with initialPin '888999'
      expect(outgoingMessages.length, equals(2));
      expect(outgoingMessages[1]['type'], equals('auth_verify'));
      expect(outgoingMessages[1]['pin'], equals('888999'));

      // Host responds with auth_ok
      mockWs.feedIncoming(jsonEncode({
        'type': 'auth_ok',
        'session_token': 'test_token_p2p',
      }));
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 100));

      // Verify UI state is authenticated
      expect(find.text('LIVE (0f)'), findsOneWidget);
      expect(find.text('Connected & Authenticated with Host PC'), findsOneWidget);
    });

    testWidgets('handles connect_error from signaling broker', (tester) async {
      final mockWs = MockWebSocket();

      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: MirrorView(
              signalingUrl: 'ws://10.0.2.2:53212',
              targetDeviceId: '999999',
              webSocketConnector: (url) async => mockWs,
            ),
          ),
        ),
      );

      await tester.pump();

      // Broker responds with connect_error
      mockWs.feedIncoming(jsonEncode({
        'type': 'connect_error',
        'reason': 'Host 999999 not found or offline',
      }));
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 100));

      // Verify SnackBar or error message is displayed
      expect(find.text('Remote Connect Error: Host 999999 not found or offline'), findsOneWidget);
      expect(find.text('OFFLINE'), findsOneWidget);
    });
  });
}
