import 'package:flutter_test/flutter_test.dart';
import 'package:vrv_desk/src/services/qr_pairing_service.dart';

void main() {
  group('QrPairingService Tests', () {
    test('serialize and deserialize JSON payload roundtrip', () {
      const data = QrPairingData(
        deviceId: '684174',
        ipAddress: '192.168.1.100',
        port: 53211,
        pin: '829104',
        signalingUrl: 'ws://127.0.0.1:53212',
      );

      final serialized = QrPairingService.serialize(data);
      expect(serialized.contains('vrv_pairing'), isTrue);

      final parsed = QrPairingService.deserialize(serialized);
      expect(parsed, isNotNull);
      expect(parsed!.deviceId, '684174');
      expect(parsed.ipAddress, '192.168.1.100');
      expect(parsed.port, 53211);
      expect(parsed.pin, '829104');
      expect(parsed.signalingUrl, 'ws://127.0.0.1:53212');
    });

    test('deserialize URI format vrvdesk://connect', () {
      const uri = 'vrvdesk://connect?id=849201&ip=10.0.2.2&port=53211&pin=123456';
      final parsed = QrPairingService.deserialize(uri);

      expect(parsed, isNotNull);
      expect(parsed!.deviceId, '849201');
      expect(parsed.ipAddress, '10.0.2.2');
      expect(parsed.port, 53211);
      expect(parsed.pin, '123456');
    });

    test('deserialize shorthand plain text format', () {
      // 6-digit ID and 6-digit PIN
      final parsedIdPin = QrPairingService.deserialize('684174:829104');
      expect(parsedIdPin, isNotNull);
      expect(parsedIdPin!.deviceId, '684174');
      expect(parsedIdPin.pin, '829104');

      // IP:port:pin
      final parsedIpPortPin = QrPairingService.deserialize('192.168.1.50:53211:999888');
      expect(parsedIpPortPin, isNotNull);
      expect(parsedIpPortPin!.ipAddress, '192.168.1.50');
      expect(parsedIpPortPin.port, 53211);
      expect(parsedIpPortPin.pin, '999888');
    });

    test('returns null on invalid garbage input', () {
      expect(QrPairingService.deserialize(''), isNull);
      expect(QrPairingService.deserialize('invalid-garbage-string'), isNull);
      expect(QrPairingService.deserialize('{"foo":"bar"}'), isNull);
    });
  });
}
