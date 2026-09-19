import 'dart:io';
import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';

/// Audio format tags supported by the protocol.
class AudioFormatTag {
  /// Standard signed 16-bit PCM little-endian.
  static const int pcmS16le = 0x01;

  /// Compressed Opus audio frame (RFC 6716).
  static const int opus = 0x02;
}

/// Information parsed from a binary VAUD packet header.
class VaudHeader {
  final int format;
  final int channels;
  final int sampleRate;
  final int payloadOffset;
  final int payloadLength;

  const VaudHeader({
    required this.format,
    required this.channels,
    required this.sampleRate,
    required this.payloadOffset,
    required this.payloadLength,
  });

  @override
  String toString() =>
      'VaudHeader(format: $format, channels: $channels, sampleRate: $sampleRate, payloadLen: $payloadLength)';
}

/// Service that demultiplexes, decodes, and feeds binary VAUD audio streams
/// to the platform's native low-latency audio player via MethodChannel.
class AudioStreamPlayer {
  static const MethodChannel _defaultChannel =
      MethodChannel('com.vrv.desk/audio');

  final MethodChannel _channel;
  final bool _enablePlatformCalls;

  int _currentSampleRate = 0;
  int _currentChannels = 0;
  bool _isInitialized = false;
  bool _isMuted = false;

  AudioStreamPlayer({
    MethodChannel? channel,
    bool? enablePlatformCalls,
  })  : _channel = channel ?? _defaultChannel,
        _enablePlatformCalls =
            enablePlatformCalls ?? (!kIsWeb && Platform.isAndroid);

  int get currentSampleRate => _currentSampleRate;
  int get currentChannels => _currentChannels;
  bool get isInitialized => _isInitialized;
  bool get isMuted => _isMuted;

  /// Checks whether a raw binary byte slice starts with the ASCII 'VAUD' magic identifier.
  static bool isVaudPacket(List<int> bytes) {
    if (bytes.length < 8) return false;
    return bytes[0] == 0x56 && // 'V'
        bytes[1] == 0x41 && // 'A'
        bytes[2] == 0x55 && // 'U'
        bytes[3] == 0x44; // 'D'
  }

  /// Parses the 8-byte VAUD header from a binary packet.
  ///
  /// Header format:
  /// - bytes 0..3: ASCII "VAUD" (0x56, 0x41, 0x55, 0x44)
  /// - byte 4: format (0x01 = PCM S16LE)
  /// - byte 5: channels (e.g. 1 or 2)
  /// - bytes 6..7: sample rate as 16-bit little-endian integer (e.g. 48000)
  /// - bytes 8..: raw PCM payload
  static VaudHeader? parseHeader(List<int> bytes) {
    if (!isVaudPacket(bytes)) return null;

    final format = bytes[4];
    final channels = bytes[5];
    final sampleRate = bytes[6] | (bytes[7] << 8);
    const payloadOffset = 8;
    final payloadLength = bytes.length - payloadOffset;

    return VaudHeader(
      format: format,
      channels: channels,
      sampleRate: sampleRate,
      payloadOffset: payloadOffset,
      payloadLength: payloadLength,
    );
  }

  /// Handles incoming VAUD packet raw bytes directly.
  /// Automatically parses the header and feeds the PCM chunk.
  Future<bool> handleVaudPacket(List<int> bytes) async {
    final header = parseHeader(bytes);
    if (header == null) return false;

    final pcmBytes = bytes is Uint8List
        ? Uint8List.sublistView(bytes, header.payloadOffset)
        : Uint8List.fromList(bytes.sublist(header.payloadOffset));

    return feedPacket(
      header.format,
      header.channels,
      header.sampleRate,
      pcmBytes,
    );
  }

  /// Ensures native AudioTrack is configured for the target sampleRate and channels.
  Future<bool> init(int sampleRate, int channels) async {
    _currentSampleRate = sampleRate;
    _currentChannels = channels;

    if (!_enablePlatformCalls) {
      _isInitialized = true;
      return true;
    }

    try {
      final res = await _channel.invokeMethod<bool>('init', {
        'sampleRate': sampleRate,
        'channels': channels,
      });
      _isInitialized = res ?? false;
      if (_isMuted) {
        await setMuted(true);
      }
      return _isInitialized;
    } catch (e) {
      debugPrint('AudioStreamPlayer init error: $e');
      _isInitialized = false;
      return false;
    }
  }

  /// Feeds a chunk of PCM audio data to the player.
  /// Automatically re-inits if sample rate or channel configuration changes.
  Future<bool> feedPacket(
    int format,
    int channels,
    int sampleRate,
    Uint8List pcm,
  ) async {
    if (pcm.isEmpty) return true;

    if (!_isInitialized ||
        _currentSampleRate != sampleRate ||
        _currentChannels != channels) {
      final ok = await init(sampleRate, channels);
      if (!ok) return false;
    }

    if (!_enablePlatformCalls) {
      return true;
    }

    try {
      final res = await _channel.invokeMethod<bool>('write', {
        'data': pcm,
        'format': format,
      });
      return res ?? false;
    } catch (e) {
      debugPrint('AudioStreamPlayer write error: $e');
      return false;
    }
  }

  /// Sets muted state.
  Future<void> setMuted(bool muted) async {
    _isMuted = muted;
    if (!_enablePlatformCalls) return;

    try {
      await _channel.invokeMethod('setMuted', {
        'muted': muted,
      });
    } catch (e) {
      debugPrint('AudioStreamPlayer setMuted error: $e');
    }
  }

  /// Stops playback and releases platform resources.
  Future<void> stop() async {
    _isInitialized = false;
    if (!_enablePlatformCalls) return;

    try {
      await _channel.invokeMethod('stop');
    } catch (e) {
      debugPrint('AudioStreamPlayer stop error: $e');
    }
  }

  /// Disposes the audio player.
  Future<void> dispose() async {
    await stop();
  }
}
