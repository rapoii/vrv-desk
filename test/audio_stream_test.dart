import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mirror_app/src/services/audio_stream_player.dart';
import 'package:mirror_app/src/views/mirror_view.dart';
import 'test_websocket_helper.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  group('AudioStreamPlayer Unit Tests', () {
    const channelName = 'com.vrv.desk/audio';
    final log = <MethodCall>[];

    setUp(() {
      log.clear();
      TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
          .setMockMethodCallHandler(const MethodChannel(channelName), (MethodCall methodCall) async {
        log.add(methodCall);
        switch (methodCall.method) {
          case 'init':
            return true;
          case 'write':
            return true;
          case 'setMuted':
            return true;
          case 'stop':
            return true;
          default:
            return null;
        }
      });
    });

    tearDown(() {
      TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
          .setMockMethodCallHandler(const MethodChannel(channelName), null);
    });

    test('isVaudPacket identifies VAUD magic bytes correctly', () {
      final validVaud = [0x56, 0x41, 0x55, 0x44, 0x01, 0x02, 0x80, 0xBB, 0x00, 0x00];
      final jpegHeader = [0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46];
      final shortBytes = [0x56, 0x41, 0x55];

      expect(AudioStreamPlayer.isVaudPacket(validVaud), isTrue);
      expect(AudioStreamPlayer.isVaudPacket(jpegHeader), isFalse);
      expect(AudioStreamPlayer.isVaudPacket(shortBytes), isFalse);
    });

    test('parseHeader parses 8-byte VAUD header fields properly', () {
      // 48000 Hz = 0xBB80 -> little-endian bytes: [0x80, 0xBB]
      final packet = [
        0x56, 0x41, 0x55, 0x44, // VAUD
        0x01,                   // Format PCM S16LE
        0x02,                   // Channels: stereo (2)
        0x80, 0xBB,             // Sample rate: 48000
        0x10, 0x20, 0x30, 0x40  // PCM payload 4 bytes
      ];

      final header = AudioStreamPlayer.parseHeader(packet);
      expect(header, isNotNull);
      expect(header!.format, equals(1));
      expect(header.channels, equals(2));
      expect(header.sampleRate, equals(48000));
      expect(header.payloadOffset, equals(8));
      expect(header.payloadLength, equals(4));
    });

    test('AudioStreamPlayer lifecycle and method channel invocation', () async {
      final player = AudioStreamPlayer(
        channel: const MethodChannel(channelName),
        enablePlatformCalls: true,
      );

      // Explicit init
      final initRes = await player.init(44100, 1);
      expect(initRes, isTrue);
      expect(player.isInitialized, isTrue);
      expect(player.currentSampleRate, equals(44100));
      expect(player.currentChannels, equals(1));
      expect(log.length, equals(1));
      expect(log[0].method, equals('init'));
      expect(log[0].arguments, equals({'sampleRate': 44100, 'channels': 1}));

      // Feed PCM directly
      final pcm = Uint8List.fromList([1, 2, 3, 4]);
      final writeRes = await player.feedPacket(1, 1, 44100, pcm);
      expect(writeRes, isTrue);
      expect(log.length, equals(2));
      expect(log[1].method, equals('write'));
      expect(log[1].arguments, equals({'data': pcm}));

      // Feed PCM with new sample rate -> should auto-reinit
      final pcm2 = Uint8List.fromList([5, 6, 7, 8]);
      await player.feedPacket(1, 2, 48000, pcm2);
      expect(player.currentSampleRate, equals(48000));
      expect(player.currentChannels, equals(2));
      expect(log.length, equals(4)); // init then write
      expect(log[2].method, equals('init'));
      expect(log[2].arguments, equals({'sampleRate': 48000, 'channels': 2}));
      expect(log[3].method, equals('write'));

      // Mute toggle
      await player.setMuted(true);
      expect(player.isMuted, isTrue);
      expect(log.last.method, equals('setMuted'));
      expect(log.last.arguments, equals({'muted': true}));

      await player.setMuted(false);
      expect(player.isMuted, isFalse);
      expect(log.last.method, equals('setMuted'));
      expect(log.last.arguments, equals({'muted': false}));

      // Stop
      await player.stop();
      expect(player.isInitialized, isFalse);
      expect(log.last.method, equals('stop'));

      await player.dispose();
    });

    test('handleVaudPacket unpacks header and invokes native write', () async {
      final player = AudioStreamPlayer(
        channel: const MethodChannel(channelName),
        enablePlatformCalls: true,
      );

      final packet = Uint8List.fromList([
        0x56, 0x41, 0x55, 0x44, // VAUD
        0x01,                   // Format PCM S16LE
        0x02,                   // Channels: stereo
        0x80, 0xBB,             // 48000
        0xAA, 0xBB, 0xCC, 0xDD  // PCM
      ]);

      final ok = await player.handleVaudPacket(packet);
      expect(ok, isTrue);
      expect(player.currentSampleRate, equals(48000));
      expect(player.currentChannels, equals(2));
      expect(log.any((call) => call.method == 'init'), isTrue);
      expect(log.any((call) => call.method == 'write'), isTrue);
    });

    test('Fallback mode when enablePlatformCalls is false (e.g. desktop/unit test default)', () async {
      final player = AudioStreamPlayer(
        channel: const MethodChannel(channelName),
        enablePlatformCalls: false,
      );

      expect(player.isInitialized, isFalse);
      final initOk = await player.init(48000, 2);
      expect(initOk, isTrue);
      expect(player.isInitialized, isTrue);

      final pcm = Uint8List.fromList([0, 1, 2, 3]);
      final feedOk = await player.feedPacket(1, 2, 48000, pcm);
      expect(feedOk, isTrue);

      await player.setMuted(true);
      expect(player.isMuted, isTrue);

      await player.stop();
      expect(player.isInitialized, isFalse);
      // No method channel calls should have occurred
      expect(log.isEmpty, isTrue);
    });
  });

  group('MirrorView Audio Integration & UI Tests', () {
    late MockWebSocket mockSocket;
    late AudioStreamPlayer mockAudioPlayer;
    final audioLog = <String>[];

    setUp(() {
      mockSocket = MockWebSocket();
      audioLog.clear();
      mockAudioPlayer = AudioStreamPlayer(
        enablePlatformCalls: false,
      );
    });

    tearDown(() async {
      // Avoid hanging on stream controller closing
    });

    testWidgets('Demuxes VAUD packets and routes to AudioStreamPlayer only when authenticated', (tester) async {
      final vaudPacket = Uint8List.fromList([
        0x56, 0x41, 0x55, 0x44, // VAUD
        0x01,                   // PCM
        0x02,                   // Stereo
        0x80, 0xBB,             // 48000
        0x01, 0x02, 0x03, 0x04  // PCM data
      ]);

      // Small 1x1 dummy JPEG
      final jpegFrame = Uint8List.fromList([
        0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46, 0x00, 0x01,
        0x01, 0x01, 0x00, 0x60, 0x00, 0x60, 0x00, 0x00, 0xFF, 0xDB, 0x00, 0x43,
        0x00, 0x08, 0x06, 0x06, 0x07, 0x06, 0x05, 0x08, 0x07, 0x07, 0x07, 0x09,
        0x09, 0x08, 0x0A, 0x0C, 0x14, 0x0D, 0x0C, 0x0B, 0x0B, 0x0C, 0x19, 0x12,
        0x13, 0x0F, 0x14, 0x1D, 0x1A, 0x1F, 0x1E, 0x1D, 0x1A, 0x1C, 0x1C, 0x20,
        0x24, 0x2E, 0x27, 0x20, 0x22, 0x2C, 0x23, 0x1C, 0x1C, 0x28, 0x37, 0x29,
        0x2C, 0x30, 0x31, 0x34, 0x34, 0x34, 0x1F, 0x27, 0x39, 0x3D, 0x38, 0x32,
        0x3C, 0x2E, 0x33, 0x34, 0x32, 0xFF, 0xC0, 0x00, 0x0B, 0x08, 0x00, 0x01,
        0x00, 0x01, 0x01, 0x01, 0x11, 0x00, 0xFF, 0xC4, 0x00, 0x1F, 0x00, 0x00,
        0x01, 0x05, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
        0x09, 0x0A, 0x0B, 0xFF, 0xDA, 0x00, 0x08, 0x01, 0x01, 0x00, 0x00, 0x3F,
        0x00, 0xBF, 0x00, 0xFF, 0xD9
      ]);

      await tester.pumpWidget(
        MaterialApp(
          home: MirrorView(
            hostIp: '192.168.1.50',
            port: 53211,
            audioPlayer: mockAudioPlayer,
            webSocketConnector: (url) async => mockSocket,
          ),
        ),
      );

      await tester.pump();

      // Before authentication, feed VAUD -> should not initialize player
      mockSocket.feedIncoming(vaudPacket);
      await tester.pump();
      expect(mockAudioPlayer.isInitialized, isFalse);

      // Authenticate
      mockSocket.feedIncoming('{"type": "auth_ok"}');
      await tester.pump();

      // Now feed VAUD -> should initialize and feed player
      mockSocket.feedIncoming(vaudPacket);
      await tester.pump();
      expect(mockAudioPlayer.isInitialized, isTrue);
      expect(mockAudioPlayer.currentSampleRate, equals(48000));
      expect(mockAudioPlayer.currentChannels, equals(2));

      // Feed JPEG frame -> verify status shows live frames
      mockSocket.feedIncoming(jpegFrame);
      await tester.pump();
      expect(find.textContaining('LIVE (1f)'), findsOneWidget);
    });

    testWidgets('Tapping Audio Mute button in floating dock toggles mute state and shows feedback', (tester) async {
      await tester.pumpWidget(
        MaterialApp(
          home: MirrorView(
            hostIp: '192.168.1.50',
            port: 53211,
            audioPlayer: mockAudioPlayer,
            webSocketConnector: (url) async => mockSocket,
          ),
        ),
      );

      await tester.pump();

      // Initially unmuted: volume_up icon
      expect(find.byIcon(Icons.volume_up), findsOneWidget);
      expect(find.byIcon(Icons.volume_off), findsNothing);
      expect(mockAudioPlayer.isMuted, isFalse);

      // Tap mute button
      await tester.tap(find.byIcon(Icons.volume_up));
      await tester.pump();

      // Now muted: volume_off icon and snackbar
      expect(find.byIcon(Icons.volume_off), findsOneWidget);
      expect(mockAudioPlayer.isMuted, isTrue);
      expect(find.text('🔇 Audio muted'), findsOneWidget);

      // Tap unmute button
      await tester.tap(find.byIcon(Icons.volume_off));
      await tester.pump();

      // Unmuted again
      expect(find.byIcon(Icons.volume_up), findsOneWidget);
      expect(mockAudioPlayer.isMuted, isFalse);
      expect(find.text('🔊 Audio unmuted'), findsOneWidget);
    });
  });
}
