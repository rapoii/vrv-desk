import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:vrv_desk/src/services/unattended_storage.dart';
import 'package:vrv_desk/src/widgets/pin_dialog.dart';
import 'package:vrv_desk/src/services/android_host_service.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  group('UnattendedStorage Unit Tests', () {
    setUp(() {
      UnattendedStorage.clearAll();
    });

    test('saves, retrieves, and removes unattended password by device ID or IP', () {
      expect(UnattendedStorage.hasSavedPassword('849201'), isFalse);
      expect(UnattendedStorage.getSavedPassword('849201'), isNull);

      UnattendedStorage.savePassword('849 201', 'mySecretPass123');
      expect(UnattendedStorage.hasSavedPassword('849201'), isTrue);
      expect(UnattendedStorage.getSavedPassword('849201'), equals('mySecretPass123'));

      UnattendedStorage.savePassword('192.168.1.50', 'admin@vrv');
      expect(UnattendedStorage.getSavedPassword('192.168.1.50'), equals('admin@vrv'));

      UnattendedStorage.removePassword('849 201');
      expect(UnattendedStorage.hasSavedPassword('849201'), isFalse);
      expect(UnattendedStorage.getSavedPassword('849201'), isNull);
      expect(UnattendedStorage.hasSavedPassword('192.168.1.50'), isTrue);
    });
  });

  group('PinDialog Unattended Access Widget Tests', () {
    testWidgets('toggles between One-Time PIN and Unattended Password modes', (tester) async {
      String? submittedSecret;
      bool? submittedRemember;

      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: PinDialog(
              deviceName: 'Workstation-PC',
              onSubmitted: (s) => submittedSecret = s,
              onSubmittedWithRemember: (s, r) {
                submittedSecret = s;
                submittedRemember = r;
              },
            ),
          ),
        ),
      );

      // Default: Dynamic PIN mode
      expect(find.text('One-Time PIN'), findsOneWidget);
      expect(find.text('Unattended'), findsOneWidget);
      expect(find.byKey(const Key('pin_input_field')), findsOneWidget);
      expect(find.byKey(const Key('unattended_password_field')), findsNothing);

      // Enter 6-digit PIN
      await tester.enterText(find.byKey(const Key('pin_input_field')), '123456');
      await tester.pump();

      // Submit button should be enabled
      final submitBtn = tester.widget<ElevatedButton>(find.byKey(const Key('pin_submit_button')));
      expect(submitBtn.onPressed, isNotNull);

      // Switch to Unattended Password mode
      await tester.tap(find.text('Unattended'));
      await tester.pump();

      expect(find.byKey(const Key('unattended_password_field')), findsOneWidget);
      expect(find.byKey(const Key('pin_input_field')), findsNothing);
      expect(find.byKey(const Key('remember_password_checkbox')), findsOneWidget);

      // Enter alphanumeric password
      await tester.enterText(find.byKey(const Key('unattended_password_field')), 'SuperSecretPass99');
      await tester.pump();

      // Toggle remember password checkbox
      await tester.tap(find.byKey(const Key('remember_password_checkbox')));
      await tester.pump();

      // Submit
      await tester.tap(find.byKey(const Key('pin_submit_button')));
      await tester.pumpAndSettle();

      expect(submittedSecret, equals('SuperSecretPass99'));
      expect(submittedRemember, isTrue);
    });
  });

  group('AndroidHostService Unattended Access Tests', () {
    late AndroidHostService hostService;

    setUp(() {
      hostService = AndroidHostService();
    });

    tearDown(() async {
      await hostService.stop();
    });

    test('authenticates client using unattended static password', () async {
      const pin = '654321';
      const staticPass = 'PermPass2026!';

      await hostService.start(
        bindAddress: InternetAddress.loopbackIPv4,
        port: 0,
        pin: pin,
        unattendedPassword: staticPass,
        enableUdpBeacon: false,
      );

      final ws = await WebSocket.connect('ws://127.0.0.1:${hostService.port}');
      final reqCompleter = Completer<void>();
      final respCompleter = Completer<Map<String, dynamic>>();

      ws.listen((data) {
        if (data is String) {
          final json = jsonDecode(data) as Map<String, dynamic>;
          if (json['type'] == 'auth_required') {
            reqCompleter.complete();
          } else {
            respCompleter.complete(json);
          }
        }
      });

      await reqCompleter.future.timeout(const Duration(seconds: 3));

      // Send auth_verify with unattended password
      ws.add(jsonEncode({
        'type': 'auth_verify',
        'pin': staticPass,
        'e2ee': false,
      }));

      final authResp = await respCompleter.future.timeout(const Duration(seconds: 3));
      expect(authResp['type'], equals('auth_ok'));
      expect(authResp['unattended'], isTrue);
      expect(hostService.authenticatedClientsCount, equals(1));

      await ws.close();
    });

    test('also allows dynamic PIN when unattended password is set', () async {
      const pin = '654321';
      const staticPass = 'PermPass2026!';

      await hostService.start(
        bindAddress: InternetAddress.loopbackIPv4,
        port: 0,
        pin: pin,
        unattendedPassword: staticPass,
        enableUdpBeacon: false,
      );

      final ws = await WebSocket.connect('ws://127.0.0.1:${hostService.port}');
      final reqCompleter = Completer<void>();
      final respCompleter = Completer<Map<String, dynamic>>();

      ws.listen((data) {
        if (data is String) {
          final json = jsonDecode(data) as Map<String, dynamic>;
          if (json['type'] == 'auth_required') {
            reqCompleter.complete();
          } else {
            respCompleter.complete(json);
          }
        }
      });

      await reqCompleter.future.timeout(const Duration(seconds: 3));

      // Send auth_verify with PIN
      ws.add(jsonEncode({
        'type': 'auth_verify',
        'pin': pin,
        'e2ee': false,
      }));

      final authResp = await respCompleter.future.timeout(const Duration(seconds: 3));
      expect(authResp['type'], equals('auth_ok'));
      expect(authResp['unattended'], isFalse);

      await ws.close();
    });

    test('rejects incorrect password when unattended access is active', () async {
      const pin = '654321';
      const staticPass = 'PermPass2026!';

      await hostService.start(
        bindAddress: InternetAddress.loopbackIPv4,
        port: 0,
        pin: pin,
        unattendedPassword: staticPass,
        enableUdpBeacon: false,
      );

      final ws = await WebSocket.connect('ws://127.0.0.1:${hostService.port}');
      final reqCompleter = Completer<void>();
      final respCompleter = Completer<Map<String, dynamic>>();

      ws.listen((data) {
        if (data is String) {
          final json = jsonDecode(data) as Map<String, dynamic>;
          if (json['type'] == 'auth_required') {
            reqCompleter.complete();
          } else {
            respCompleter.complete(json);
          }
        }
      });

      await reqCompleter.future.timeout(const Duration(seconds: 3));

      // Send wrong password
      ws.add(jsonEncode({
        'type': 'auth_verify',
        'pin': 'WrongPass123',
        'e2ee': false,
      }));

      final authResp = await respCompleter.future.timeout(const Duration(seconds: 3));
      expect(authResp['type'], equals('auth_failed'));
      expect(authResp['reason'], contains('Invalid PIN or Unattended Password'));

      await ws.close();
    });
  });
}
