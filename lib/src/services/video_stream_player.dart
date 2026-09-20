import 'dart:typed_data';
import 'package:flutter/services.dart';

enum VideoFrameType {
  idr,
  delta,
  unknown,
}

class Vh24Packet {
  final VideoFrameType frameType;
  final int sequenceNumber;
  final Uint8List nalPayload;

  const Vh24Packet({
    required this.frameType,
    required this.sequenceNumber,
    required this.nalPayload,
  });
}

class VideoStreamPlayer {
  static const MethodChannel _channel = MethodChannel('com.vrv.desk/video');

  int? _textureId;
  bool _isInitialized = false;

  int? get textureId => _textureId;
  bool get isInitialized => _isInitialized;

  /// Check whether a binary WebSocket packet has the VH24 magic header
  static bool isVh24Packet(List<int> data) {
    if (data.length < 8) return false;
    return data[0] == 0x56 && // 'V'
        data[1] == 0x48 && // 'H'
        data[2] == 0x32 && // '2'
        data[3] == 0x34; // '4'
  }

  /// Parse a VH24 binary packet into a Vh24Packet structure
  static Vh24Packet? parseVh24Packet(List<int> data) {
    if (!isVh24Packet(data)) return null;

    final typeByte = data[4];
    final frameType = typeByte == 0x01
        ? VideoFrameType.idr
        : (typeByte == 0x02 ? VideoFrameType.delta : VideoFrameType.unknown);

    final seq = (data[6] << 8) | data[7];
    final nalPayload = Uint8List.fromList(data.sublist(8));

    return Vh24Packet(
      frameType: frameType,
      sequenceNumber: seq,
      nalPayload: nalPayload,
    );
  }

  /// Initialize the Android native MediaCodec H.264 decoder and Flutter SurfaceTexture
  Future<int?> init({int width = 1280, int height = 720}) async {
    try {
      final res = await _channel.invokeMethod<int>('init', {
        'width': width,
        'height': height,
      });
      _textureId = res;
      _isInitialized = res != null;
      return _textureId;
    } catch (_) {
      _isInitialized = false;
      return null;
    }
  }

  /// Write a raw H.264 NAL bitstream chunk to the native hardware decoder
  Future<bool> write(Uint8List nalData) async {
    if (!_isInitialized) return false;
    try {
      final res = await _channel.invokeMethod<bool>('write', {
        'data': nalData,
      });
      return res ?? false;
    } catch (_) {
      return false;
    }
  }

  /// Dispose the decoder and release the SurfaceTexture
  Future<void> dispose() async {
    if (_isInitialized) {
      try {
        await _channel.invokeMethod<void>('dispose');
      } catch (_) {}
      _textureId = null;
      _isInitialized = false;
    }
  }
}
