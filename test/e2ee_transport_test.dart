import 'dart:convert';
import 'dart:typed_data';
import 'package:flutter_test/flutter_test.dart';
import 'package:vrv_desk/src/services/e2ee_transport.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  group('E2eeTransportSession Tests', () {
    test('isE2eePacket detects VE2E magic correctly', () {
      final valid = Uint8List(28);
      valid[0] = 0x56; // 'V'
      valid[1] = 0x45; // 'E'
      valid[2] = 0x32; // '2'
      valid[3] = 0x45; // 'E'
      expect(E2eeTransportSession.isE2eePacket(valid), isTrue);

      final invalidMagic = Uint8List(28);
      expect(E2eeTransportSession.isE2eePacket(invalidMagic), isFalse);

      final tooShort = Uint8List.fromList([0x56, 0x45, 0x32, 0x45]);
      expect(E2eeTransportSession.isE2eePacket(tooShort), isFalse);
    });

    test('encrypt and decrypt roundtrip matches plaintext', () async {
      const token = 'test_token_1234567890abcdef12345678';
      final sender = E2eeTransportSession.fromToken(token);
      final receiver = E2eeTransportSession.fromToken(token);

      final originalData = utf8.encode('Hello Secure Screen Mirroring!');
      final encryptedPacket = await sender.encrypt(Uint8List.fromList(originalData));

      expect(E2eeTransportSession.isE2eePacket(encryptedPacket), isTrue);
      expect(encryptedPacket.length, equals(4 + 8 + originalData.length + 16));

      final decryptedData = await receiver.decrypt(encryptedPacket);
      expect(utf8.decode(decryptedData), equals('Hello Secure Screen Mirroring!'));
    });

    test('tampered ciphertext fails decryption with exception', () async {
      const token = 'tamper_test_token';
      final sender = E2eeTransportSession.fromToken(token);
      final receiver = E2eeTransportSession.fromToken(token);

      final packet = await sender.encrypt(Uint8List.fromList([1, 2, 3, 4, 5]));
      // Tamper ciphertext
      packet[15] ^= 0xFF;

      expect(() async => await receiver.decrypt(packet), throwsException);
    });
  });
}
