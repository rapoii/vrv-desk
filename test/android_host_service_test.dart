import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:vrv_desk/src/services/android_host_service.dart';
import 'package:vrv_desk/src/services/e2ee_transport.dart';
import 'package:vrv_desk/src/services/lan_discovery_service.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  const accessibilityChannelName = 'com.vrv.desk/accessibility';
  const captureChannelName = 'com.vrv.desk/capture';
  const adbBridgeChannelName = 'com.vrv.desk/adb_bridge';
  final accessibilityCalls = <MethodCall>[];
  final captureCalls = <MethodCall>[];
  final adbBridgeCalls = <MethodCall>[];

  setUp(() {
    accessibilityCalls.clear();
    captureCalls.clear();
    adbBridgeCalls.clear();
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(
      const MethodChannel(accessibilityChannelName),
      (MethodCall methodCall) async {
        accessibilityCalls.add(methodCall);
        switch (methodCall.method) {
          case 'isAccessibilityEnabled':
            return true;
          case 'openAccessibilitySettings':
            return true;
          case 'tap':
            return true;
          case 'swipe':
            return true;
          case 'globalAction':
            return true;
          default:
            return null;
        }
      },
    );

    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(
      const MethodChannel(captureChannelName),
      (MethodCall methodCall) async {
        captureCalls.add(methodCall);
        switch (methodCall.method) {
          case 'isInternalAudioSupported':
            return true;
          case 'startCapture':
            return true;
          case 'stopCapture':
            return true;
          case 'isCapturing':
            return true;
          default:
            return null;
        }
      },
    );

    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(
      const MethodChannel(adbBridgeChannelName),
      (MethodCall methodCall) async {
        adbBridgeCalls.add(methodCall);
        switch (methodCall.method) {
          case 'getAdbStatus':
            return {
              'adb_enabled': true,
              'port_5555_open': true,
              'max_refresh_rate': 120.0,
              'shell_ready': true,
              'high_performance_available': true,
            };
          case 'tap':
          case 'swipe':
          case 'globalAction':
          case 'keyevent':
          case 'text':
            return true;
          default:
            return null;
        }
      },
    );
  });

  tearDown(() {
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(
      const MethodChannel(accessibilityChannelName),
      null,
    );
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(
      const MethodChannel(captureChannelName),
      null,
    );
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(
      const MethodChannel(adbBridgeChannelName),
      null,
    );
  });

  group('AndroidHostService Tests', () {
    late AndroidHostService hostService;
    StreamController<Uint8List>? frameController;

    setUp(() {
      frameController = StreamController<Uint8List>.broadcast();
      hostService = AndroidHostService(
        videoStreamOverride: frameController!.stream,
        accessibilityChannel: const MethodChannel(accessibilityChannelName),
        screenWidth: 1080,
        screenHeight: 2400,
      );
    });

    tearDown(() async {
      await hostService.stop();
      await frameController?.close();
    });

    test('Lifecycle: start and stop server updates state and generates PIN', () async {
      expect(hostService.isRunning, isFalse);
      expect(hostService.currentPin, isEmpty);

      // Start on ephemeral loopback port for test
      await hostService.start(
        bindAddress: InternetAddress.loopbackIPv4,
        port: 0,
        pin: '123456',
        deviceId: 'android_test_1',
        deviceName: 'Pixel-Host-Test',
        enableUdpBeacon: false,
      );

      expect(hostService.isRunning, isTrue);
      expect(hostService.currentPin, equals('123456'));
      expect(hostService.port, greaterThan(0));
      expect(hostService.deviceId, equals('android_test_1'));
      expect(hostService.deviceName, equals('Pixel-Host-Test'));

      await hostService.stop();
      expect(hostService.isRunning, isFalse);
    });

    test('Generates random 6-digit PIN when none provided', () async {
      await hostService.start(
        bindAddress: InternetAddress.loopbackIPv4,
        port: 0,
        enableUdpBeacon: false,
      );

      expect(hostService.currentPin.length, equals(6));
      expect(int.tryParse(hostService.currentPin), isNotNull);
      expect(int.parse(hostService.currentPin), inInclusiveRange(100000, 999999));
    });

    test('Handshake: successful PIN verification with E2EE session derivation', () async {
      await hostService.start(
        bindAddress: InternetAddress.loopbackIPv4,
        port: 0,
        pin: '654321',
        enableUdpBeacon: false,
      );

      final ws = await WebSocket.connect('ws://127.0.0.1:${hostService.port}');
      final messages = <Map<String, dynamic>>[];
      final completerAuthOk = Completer<Map<String, dynamic>>();

      final sub = ws.listen((data) {
        if (data is String) {
          final json = jsonDecode(data) as Map<String, dynamic>;
          messages.add(json);
          if (json['type'] == 'auth_ok') {
            completerAuthOk.complete(json);
          }
        }
      });

      // 1. Host should automatically send auth_required
      await Future.delayed(const Duration(milliseconds: 50));
      expect(messages.isNotEmpty, isTrue);
      expect(messages.first['type'], equals('auth_required'));

      // 2. Client replies with valid PIN and e2ee requested
      ws.add(jsonEncode({
        'type': 'auth_verify',
        'pin': '654321',
        'e2ee': true,
      }));

      final authOk = await completerAuthOk.future.timeout(const Duration(seconds: 3));
      expect(authOk['type'], equals('auth_ok'));
      expect(authOk['session_token'], isNotNull);
      expect(authOk['e2ee'], isTrue);
      expect(hostService.authenticatedClientsCount, equals(1));

      await sub.cancel();
      await ws.close();
    });

    test('Handshake: invalid PIN returns auth_failed and disconnects on exhaustion', () async {
      await hostService.start(
        bindAddress: InternetAddress.loopbackIPv4,
        port: 0,
        pin: '999888',
        enableUdpBeacon: false,
      );

      final ws = await WebSocket.connect('ws://127.0.0.1:${hostService.port}');
      final authFailedMessages = <Map<String, dynamic>>[];
      final closeCompleter = Completer<void>();

      ws.listen(
        (data) {
          if (data is String) {
            final json = jsonDecode(data) as Map<String, dynamic>;
            if (json['type'] == 'auth_failed') {
              authFailedMessages.add(json);
            }
          }
        },
        onDone: () {
          if (!closeCompleter.isCompleted) closeCompleter.complete();
        },
      );

      await Future.delayed(const Duration(milliseconds: 50));

      // Attempt 1: wrong PIN
      ws.add(jsonEncode({'type': 'auth_verify', 'pin': '000000'}));
      await Future.delayed(const Duration(milliseconds: 50));
      expect(authFailedMessages.length, equals(1));
      expect(authFailedMessages.last['remaining_attempts'], equals(2));

      // Attempt 2: wrong PIN
      ws.add(jsonEncode({'type': 'auth_verify', 'pin': '000001'}));
      await Future.delayed(const Duration(milliseconds: 50));
      expect(authFailedMessages.length, equals(2));
      expect(authFailedMessages.last['remaining_attempts'], equals(1));

      // Attempt 3: wrong PIN -> connection should be closed
      ws.add(jsonEncode({'type': 'auth_verify', 'pin': '000002'}));
      await closeCompleter.future.timeout(const Duration(seconds: 3));

      expect(authFailedMessages.length, equals(3));
      expect(authFailedMessages.last['remaining_attempts'], equals(0));
      expect(hostService.authenticatedClientsCount, equals(0));
    });

    test('Broadcasting: broadcasts E2EE encrypted VH24 frames to authenticated clients', () async {
      await hostService.start(
        bindAddress: InternetAddress.loopbackIPv4,
        port: 0,
        pin: '112233',
        enableUdpBeacon: false,
      );

      final ws = await WebSocket.connect('ws://127.0.0.1:${hostService.port}');
      String? sessionToken;
      final binaryPackets = <Uint8List>[];

      final sub = ws.listen((data) {
        if (data is String) {
          final json = jsonDecode(data) as Map<String, dynamic>;
          if (json['type'] == 'auth_ok') {
            sessionToken = json['session_token'] as String?;
          }
        } else if (data is List<int>) {
          binaryPackets.add(Uint8List.fromList(data));
        }
      });

      // Authenticate
      ws.add(jsonEncode({
        'type': 'auth_verify',
        'pin': '112233',
        'e2ee': true,
      }));

      // Wait for auth_ok
      await Future.delayed(const Duration(milliseconds: 100));
      expect(sessionToken, isNotNull);

      final clientE2ee = E2eeTransportSession.fromToken(sessionToken!);

      // Create a mock VH24 frame: [V, H, 2, 4, len, flags, seq, payload...]
      final mockVh24 = Uint8List.fromList([
        0x56, 0x48, 0x32, 0x34, // "VH24"
        0x00, 0x00, 0x00, 0x04, // len = 4
        0x01,                   // keyframe
        0x00, 0x00, 0x01,       // seq
        0xAA, 0xBB, 0xCC, 0xDD  // NAL
      ]);

      // Push frame into host
      frameController!.add(mockVh24);
      await Future.delayed(const Duration(milliseconds: 100));

      expect(binaryPackets.length, equals(1));
      final receivedEncrypted = binaryPackets.first;

      // Ensure it is encapsulated in E2EE packet (magic 'VE2E')
      expect(E2eeTransportSession.isE2eePacket(receivedEncrypted), isTrue);

      // Decrypt using client's derived session key
      final decryptedFrame = await clientE2ee.decrypt(receivedEncrypted);
      expect(decryptedFrame, equals(mockVh24));
      expect(hostService.framesSent, equals(1));

      await sub.cancel();
      await ws.close();
    });

    test('Remote Input: dispatches decrypted touch_tap, swipe, and shortcut to accessibility channel', () async {
      await hostService.start(
        bindAddress: InternetAddress.loopbackIPv4,
        port: 0,
        pin: '445566',
        enableUdpBeacon: false,
      );

      final ws = await WebSocket.connect('ws://127.0.0.1:${hostService.port}');
      String? sessionToken;

      ws.listen((data) {
        if (data is String) {
          final json = jsonDecode(data) as Map<String, dynamic>;
          if (json['type'] == 'auth_ok') {
            sessionToken = json['session_token'] as String?;
          }
        }
      });

      // Authenticate with E2EE
      ws.add(jsonEncode({
        'type': 'auth_verify',
        'pin': '445566',
        'e2ee': true,
      }));

      await Future.delayed(const Duration(milliseconds: 100));
      expect(sessionToken, isNotNull);

      final clientE2ee = E2eeTransportSession.fromToken(sessionToken!);

      // 1. Send encrypted touch_tap event with normalized coordinates
      final tapPayload = jsonEncode({
        'type': 'touch_tap',
        'x': 0.5, // 0.5 * 1080 = 540
        'y': 0.25, // 0.25 * 2400 = 600
      });
      final encTap = await clientE2ee.encrypt(Uint8List.fromList(utf8.encode(tapPayload)));
      ws.add(encTap);

      await Future.delayed(const Duration(milliseconds: 100));

      expect(accessibilityCalls.any((c) => c.method == 'tap'), isTrue);
      final tapCall = accessibilityCalls.firstWhere((c) => c.method == 'tap');
      expect((tapCall.arguments['x'] as num).toDouble(), closeTo(540.0, 1.0));
      expect((tapCall.arguments['y'] as num).toDouble(), closeTo(600.0, 1.0));

      // 2. Send swipe event
      final swipePayload = jsonEncode({
        'type': 'swipe',
        'x1': 0.1,
        'y1': 0.8,
        'x2': 0.1,
        'y2': 0.2,
        'duration': 250,
      });
      final encSwipe = await clientE2ee.encrypt(Uint8List.fromList(utf8.encode(swipePayload)));
      ws.add(encSwipe);

      await Future.delayed(const Duration(milliseconds: 100));

      expect(accessibilityCalls.any((c) => c.method == 'swipe'), isTrue);
      final swipeCall = accessibilityCalls.firstWhere((c) => c.method == 'swipe');
      expect((swipeCall.arguments['x1'] as num).toDouble(), closeTo(108.0, 1.0));
      expect((swipeCall.arguments['y1'] as num).toDouble(), closeTo(1920.0, 1.0));
      expect((swipeCall.arguments['x2'] as num).toDouble(), closeTo(108.0, 1.0));
      expect((swipeCall.arguments['y2'] as num).toDouble(), closeTo(480.0, 1.0));
      expect(swipeCall.arguments['duration'], equals(250));

      // 3. Send shortcut event ('back')
      final shortcutPayload = jsonEncode({
        'type': 'shortcut',
        'name': 'back',
      });
      final encShortcut = await clientE2ee.encrypt(Uint8List.fromList(utf8.encode(shortcutPayload)));
      ws.add(encShortcut);

      await Future.delayed(const Duration(milliseconds: 100));

      expect(accessibilityCalls.any((c) => c.method == 'globalAction'), isTrue);
      final globalCall = accessibilityCalls.firstWhere((c) => c.method == 'globalAction');
      expect(globalCall.arguments['action'], equals('back'));

      await ws.close();
    });

    test('Discovery Beacon: generates and serializes valid LanDiscovery beacon bytes', () {
      final beaconBytes = hostService.generateBeaconPacket(
        deviceId: 'android_dev_99',
        deviceName: 'Pixel-Host',
        port: 53211,
      );

      final decoded = jsonDecode(utf8.decode(beaconBytes)) as Map<String, dynamic>;
      expect(decoded['device_id'], equals('android_dev_99'));
      expect(decoded['device_name'], equals('Pixel-Host'));
      expect(decoded['os_type'], equals('android'));
      expect(decoded['port'], equals(53211));
      expect(decoded['protocol_version'], equals(1));
    });

    test('Audio Capture API: isInternalAudioSupported and startCapture pass audio flag', () async {
      final supported = await hostService.isInternalAudioSupported();
      expect(supported, isTrue);
      expect(captureCalls.any((c) => c.method == 'isInternalAudioSupported'), isTrue);

      await hostService.startCapture(enableAudio: true);
      final startCall = captureCalls.firstWhere((c) => c.method == 'startCapture');
      expect(startCall.arguments['enableAudio'], isTrue);
    });

    test('Multiplexed Streaming: broadcasts binary VAUD audio alongside VH24 video over E2EE', () async {
      await hostService.start(
        bindAddress: InternetAddress.loopbackIPv4,
        port: 0,
        pin: '123456',
        enableUdpBeacon: false,
      );

      final ws = await WebSocket.connect('ws://127.0.0.1:${hostService.port}');
      final msgCompleter = Completer<String>();
      final receivedFrames = <Uint8List>[];

      late E2eeTransportSession clientE2ee;

      final sub = ws.listen((data) async {
        if (data is String) {
          final json = jsonDecode(data);
          if (json['type'] == 'auth_required') {
            final authVerify = jsonEncode({
              'type': 'auth_verify',
              'pin': '123456',
              'e2ee': true,
            });
            ws.add(authVerify);
          } else if (json['type'] == 'auth_ok') {
            clientE2ee = E2eeTransportSession.fromToken(json['session_token']);
            msgCompleter.complete('auth_ok');
          }
        } else if (data is List<int>) {
          final decrypted = await clientE2ee.decrypt(Uint8List.fromList(data));
          receivedFrames.add(decrypted);
        }
      });

      await msgCompleter.future.timeout(const Duration(seconds: 2));

      // 1. Emit VH24 video frame
      final vh24Packet = Uint8List.fromList([
        0x56, 0x48, 0x32, 0x34, // "VH24"
        0x00, 0x00, 0x00, 0x04, // length: 4
        0x01,                   // keyframe
        0x00, 0x00, 0x00, 0x01, // seq: 1
        0x65, 0x88, 0x84, 0x00, // payload
      ]);
      frameController!.add(vh24Packet);

      // 2. Emit VAUD audio frame
      final vaudPacket = Uint8List.fromList([
        0x56, 0x41, 0x55, 0x44, // "VAUD"
        0x01,                   // format: PCM S16LE
        0x02,                   // channels: stereo
        0x80, 0xBB,             // 48000 Hz little-endian
        0x00, 0x01, 0x02, 0x03, // PCM audio payload
      ]);
      frameController!.add(vaudPacket);

      await Future.delayed(const Duration(milliseconds: 150));

      expect(receivedFrames.length, equals(2));
      // Verify first frame is VH24
      expect(receivedFrames[0].sublist(0, 4), equals([0x56, 0x48, 0x32, 0x34]));
      // Verify second frame is VAUD audio
      expect(receivedFrames[1].sublist(0, 4), equals([0x56, 0x41, 0x55, 0x44]));
      expect(receivedFrames[1][4], equals(0x01)); // PCM
      expect(receivedFrames[1][5], equals(0x02)); // Stereo
      expect(receivedFrames[1][6] | (receivedFrames[1][7] << 8), equals(48000));

      await sub.cancel();
      await ws.close();
    });

    test('ADB Bridge API: queries getAdbStatus and detects high refresh rate capability', () async {
      final status = await hostService.getAdbStatus();
      expect(status, isNotNull);
      expect(status!['high_performance_available'], isTrue);
      expect(status['max_refresh_rate'], equals(120.0));

      final isHighPerf = await hostService.isAdbHighPerformanceAvailable();
      expect(isHighPerf, isTrue);

      final refreshRate = await hostService.getMaxDisplayRefreshRate();
      expect(refreshRate, equals(120.0));
    });

    test('Dual Input Routing: routes touch and swipe through AdbInputBridge when enabled', () async {
      hostService.useAdbInputBridge = true;

      await hostService.start(
        bindAddress: InternetAddress.loopbackIPv4,
        port: 0,
        pin: '123456',
        enableUdpBeacon: false,
      );

      final ws = await WebSocket.connect('ws://127.0.0.1:${hostService.port}');
      final msgCompleter = Completer<String>();
      late E2eeTransportSession clientE2ee;

      ws.listen((data) {
        if (data is String) {
          final json = jsonDecode(data);
          if (json['type'] == 'auth_required') {
            ws.add(jsonEncode({'type': 'auth_verify', 'pin': '123456', 'e2ee': true}));
          } else if (json['type'] == 'auth_ok') {
            clientE2ee = E2eeTransportSession.fromToken(json['session_token']);
            msgCompleter.complete('auth_ok');
          }
        }
      });

      await msgCompleter.future.timeout(const Duration(seconds: 2));

      // Send touch_down through ADB bridge
      final touchPayload = jsonEncode({'type': 'touch_down', 'x': 0.5, 'y': 0.5});
      final encTouch = await clientE2ee.encrypt(Uint8List.fromList(utf8.encode(touchPayload)));
      ws.add(encTouch);

      await Future.delayed(const Duration(milliseconds: 100));

      expect(adbBridgeCalls.any((c) => c.method == 'tap'), isTrue);
      final tapCall = adbBridgeCalls.firstWhere((c) => c.method == 'tap');
      expect((tapCall.arguments['x'] as num).toDouble(), closeTo(540.0, 1.0));
      expect((tapCall.arguments['y'] as num).toDouble(), closeTo(1200.0, 1.0));

      // Send swipe through ADB bridge
      final swipePayload = jsonEncode({
        'type': 'swipe',
        'x1': 0.2,
        'y1': 0.8,
        'x2': 0.2,
        'y2': 0.2,
        'duration': 150,
      });
      final encSwipe = await clientE2ee.encrypt(Uint8List.fromList(utf8.encode(swipePayload)));
      ws.add(encSwipe);

      await Future.delayed(const Duration(milliseconds: 100));

      expect(adbBridgeCalls.any((c) => c.method == 'swipe'), isTrue);
      final swipeCall = adbBridgeCalls.firstWhere((c) => c.method == 'swipe');
      expect((swipeCall.arguments['x1'] as num).toDouble(), closeTo(216.0, 1.0));
      expect((swipeCall.arguments['y1'] as num).toDouble(), closeTo(1920.0, 1.0));
      expect((swipeCall.arguments['x2'] as num).toDouble(), closeTo(216.0, 1.0));
      expect((swipeCall.arguments['y2'] as num).toDouble(), closeTo(480.0, 1.0));
      expect(swipeCall.arguments['duration'], equals(150));

      // Send shortcut through ADB bridge
      final shortcutPayload = jsonEncode({'type': 'shortcut', 'action': 'home'});
      final encShortcut = await clientE2ee.encrypt(Uint8List.fromList(utf8.encode(shortcutPayload)));
      ws.add(encShortcut);

      await Future.delayed(const Duration(milliseconds: 100));

      expect(adbBridgeCalls.any((c) => c.method == 'globalAction'), isTrue);
      final globalCall = adbBridgeCalls.firstWhere((c) => c.method == 'globalAction');
      expect(globalCall.arguments['action'], equals('home'));

      await ws.close();
    });
  });
}
