import 'dart:convert';
import 'dart:typed_data';
import 'package:crypto/crypto.dart' as crypto;
import 'package:cryptography/cryptography.dart';

class E2eeTransportSession {
  static const List<int> magic = [0x56, 0x45, 0x32, 0x45]; // 'VE2E'

  final SecretKey _secretKey;
  final Cipher _algorithm;
  int _sendSeq = 0;
  int _recvSeq = 0;

  E2eeTransportSession._(this._secretKey) : _algorithm = Chacha20.poly1305Aead();

  factory E2eeTransportSession.fromToken(String token) {
    final salt = utf8.encode('VRV_DESK_E2EE_KEY_SALT_v1:$token');
    final hash = crypto.sha256.convert(salt).bytes;
    final secretKey = SecretKey(hash);
    return E2eeTransportSession._(secretKey);
  }

  static bool isE2eePacket(List<int> data) {
    if (data.length < 28) return false;
    return data[0] == magic[0] &&
        data[1] == magic[1] &&
        data[2] == magic[2] &&
        data[3] == magic[3];
  }

  Future<Uint8List> encrypt(Uint8List plaintext) async {
    final seqBytes = Uint8List(8);
    final bdata = ByteData.sublistView(seqBytes);
    bdata.setUint64(0, _sendSeq, Endian.little);

    final nonce = Uint8List(12);
    nonce.setRange(0, 8, seqBytes);

    final secretBox = await _algorithm.encrypt(
      plaintext,
      secretKey: _secretKey,
      nonce: nonce,
    );

    final ciphertext = secretBox.cipherText;
    final mac = secretBox.mac.bytes;

    final out = Uint8List(4 + 8 + ciphertext.length + mac.length);
    out.setRange(0, 4, magic);
    out.setRange(4, 12, seqBytes);
    out.setRange(12, 12 + ciphertext.length, ciphertext);
    out.setRange(12 + ciphertext.length, out.length, mac);

    _sendSeq++;
    return out;
  }

  Future<Uint8List> decrypt(Uint8List packet) async {
    if (!isE2eePacket(packet)) {
      throw FormatException('Invalid E2EE packet magic or header');
    }

    final seqBytes = packet.sublist(4, 12);
    final bdata = ByteData.sublistView(seqBytes);
    final seq = bdata.getUint64(0, Endian.little);

    final nonce = Uint8List(12);
    nonce.setRange(0, 8, seqBytes);

    final cipherLen = packet.length - 28;
    final ciphertext = packet.sublist(12, 12 + cipherLen);
    final macBytes = packet.sublist(12 + cipherLen);

    final secretBox = SecretBox(
      ciphertext,
      nonce: nonce,
      mac: Mac(macBytes),
    );

    final plain = await _algorithm.decrypt(
      secretBox,
      secretKey: _secretKey,
    );

    _recvSeq = seq;
    return Uint8List.fromList(plain);
  }
}
