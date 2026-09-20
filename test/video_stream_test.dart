import 'dart:typed_data';
import 'package:flutter_test/flutter_test.dart';
import 'package:vrv_desk/src/services/video_stream_player.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  group('VideoStreamPlayer and VH24 Packet Demuxing Tests', () {
    test('isVh24Packet identifies valid VH24 header and rejects invalid ones', () {
      final validVh24 = Uint8List.fromList([
        0x56, 0x48, 0x32, 0x34, // "VH24"
        0x01,                   // Frame Type IDR
        0x00,                   // Flags
        0x00, 0x01,             // Seq 1
        0x00, 0x00, 0x00, 0x01, // Annex-B start
        0x67,                   // SPS
      ]);

      final invalidShort = Uint8List.fromList([0x56, 0x48, 0x32]);
      final invalidMagic = Uint8List.fromList([0x56, 0x41, 0x55, 0x44, 1, 2, 0, 0]); // "VAUD"
      final jpegHeader = Uint8List.fromList([0xFF, 0xD8, 0xFF, 0xE0]);

      expect(VideoStreamPlayer.isVh24Packet(validVh24), isTrue);
      expect(VideoStreamPlayer.isVh24Packet(invalidShort), isFalse);
      expect(VideoStreamPlayer.isVh24Packet(invalidMagic), isFalse);
      expect(VideoStreamPlayer.isVh24Packet(jpegHeader), isFalse);
    });

    test('parseVh24Packet extracts frame type, sequence, and NAL payload correctly', () {
      final packet = Uint8List.fromList([
        0x56, 0x48, 0x32, 0x34, // "VH24"
        0x02,                   // Frame Type DELTA
        0x00,                   // Flags
        0x01, 0x2C,             // Seq 300 (0x012C)
        0x00, 0x00, 0x00, 0x01, // NAL prefix
        0x41, 0x9A, 0xBC,       // Slice payload
      ]);

      final parsed = VideoStreamPlayer.parseVh24Packet(packet);
      expect(parsed, isNotNull);
      expect(parsed!.frameType, equals(VideoFrameType.delta));
      expect(parsed.sequenceNumber, equals(300));
      expect(parsed.nalPayload, equals(Uint8List.fromList([0, 0, 0, 1, 0x41, 0x9A, 0xBC])));
    });
  });
}
