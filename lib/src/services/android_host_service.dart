import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'dart:math';
import 'dart:typed_data';

import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';

import 'e2ee_transport.dart';
import 'lan_discovery_service.dart';

/// Representation of a connected client to the Android Host
class _HostClient {
  final WebSocket socket;
  bool isAuthenticated = false;
  int remainingAttempts = 3;
  String? sessionToken;
  bool isE2ee = false;
  E2eeTransportSession? e2eeSession;
  StreamSubscription? subscription;

  _HostClient(this.socket);
}

/// Service managing the Android Host streaming server, WebSocket connections,
/// dynamic PIN authentication, ChaCha20-Poly1305 E2EE encryption, UDP discovery beacon,
/// and remote input accessibility dispatch.
class AndroidHostService {
  static const int defaultPort = 53211;
  static const int defaultUdpPort = 53210;
  static const String defaultMulticastAddress = LanDiscoveryService.multicastAddress;

  static const String captureStreamChannelName = 'com.vrv.desk/capture_stream';
  static const String accessibilityChannelName = 'com.vrv.desk/accessibility';
  static const String captureChannelName = 'com.vrv.desk/capture';
  static const String adbBridgeChannelName = 'com.vrv.desk/adb_bridge';

  final MethodChannel _accessibilityChannel;
  final MethodChannel _captureChannel;
  final MethodChannel _adbBridgeChannel;
  final EventChannel? _captureStreamChannel;
  final Stream<Uint8List>? _videoStreamOverride;

  bool useAdbInputBridge;
  double screenWidth;
  double screenHeight;

  HttpServer? _server;
  RawDatagramSocket? _udpSocket;
  Timer? _beaconTimer;
  StreamSubscription? _videoSubscription;
  Timer? _clipboardTimer;
  String? _lastSentClipboard;

  final Set<_HostClient> _clients = {};

  bool _isRunning = false;
  String _currentPin = '';
  String? _unattendedPassword;
  int _port = defaultPort;
  String _deviceId = '';
  String _deviceName = '';
  int _framesSent = 0;

  AndroidHostService({
    MethodChannel? accessibilityChannel,
    MethodChannel? captureChannel,
    MethodChannel? adbBridgeChannel,
    EventChannel? captureStreamChannel,
    Stream<Uint8List>? videoStreamOverride,
    this.screenWidth = 1080.0,
    this.screenHeight = 2400.0,
    this.useAdbInputBridge = false,
  })  : _accessibilityChannel =
            accessibilityChannel ?? const MethodChannel(accessibilityChannelName),
        _captureChannel = captureChannel ?? const MethodChannel(captureChannelName),
        _adbBridgeChannel = adbBridgeChannel ?? const MethodChannel(adbBridgeChannelName),
        _captureStreamChannel =
            captureStreamChannel ?? (videoStreamOverride == null ? const EventChannel(captureStreamChannelName) : null),
        _videoStreamOverride = videoStreamOverride;

  bool get isRunning => _isRunning;
  String get currentPin => _currentPin;
  String? get unattendedPassword => _unattendedPassword;
  int get port => _port;
  String get deviceId => _deviceId;
  String get deviceName => _deviceName;
  int get authenticatedClientsCount =>
      _clients.where((c) => c.isAuthenticated).length;
  int get framesSent => _framesSent;

  /// Generate a random 6-digit PIN (100000 - 999999)
  static String generatePin() {
    final rand = Random.secure();
    return (100000 + rand.nextInt(900000)).toString();
  }

  /// Generate a random device ID
  static String generateDeviceId() {
    final rand = Random.secure();
    return (100000 + rand.nextInt(900000)).toString();
  }

  /// Starts the Android Host service.
  Future<void> start({
    InternetAddress? bindAddress,
    int port = defaultPort,
    String? pin,
    String? unattendedPassword,
    String? deviceId,
    String? deviceName,
    bool enableUdpBeacon = true,
  }) async {
    if (_isRunning) return;

    _port = port;
    _currentPin = pin ?? generatePin();
    _unattendedPassword = unattendedPassword;
    _deviceId = deviceId ?? generateDeviceId();
    _deviceName = deviceName ?? 'Android-Host';
    _framesSent = 0;

    final address = bindAddress ?? InternetAddress.anyIPv4;
    _server = await HttpServer.bind(address, _port);
    _port = _server!.port;

    _server!.listen(_handleHttpRequest);
    _isRunning = true;

    // Start video stream subscription
    _startVideoStream();

    // Start UDP discovery beacon
    if (enableUdpBeacon) {
      await _startUdpBeacon();
    }

    // Start Bi-directional Clipboard sync monitor
    _startClipboardMonitoring();
  }

  /// Stops the Android Host service.
  Future<void> stop() async {
    if (!_isRunning) return;
    _isRunning = false;

    // Stop clipboard monitoring
    _clipboardTimer?.cancel();
    _clipboardTimer = null;

    // Stop UDP beacon
    _beaconTimer?.cancel();
    _beaconTimer = null;
    _udpSocket?.close();
    _udpSocket = null;

    // Stop video subscription
    await _videoSubscription?.cancel();
    _videoSubscription = null;

    // Close all connected clients
    for (final client in _clients.toList()) {
      try {
        await client.subscription?.cancel();
        await client.socket.close(WebSocketStatus.goingAway, 'Host stopped');
      } catch (_) {}
    }
    _clients.clear();

    // Close HTTP server
    await _server?.close(force: true);
    _server = null;

    _currentPin = '';
  }

  void _handleHttpRequest(HttpRequest request) {
    if (WebSocketTransformer.isUpgradeRequest(request)) {
      WebSocketTransformer.upgrade(request).then(_handleWebSocketClient).catchError((e) {
        debugPrint('[AndroidHostService] WebSocket upgrade error: $e');
      });
    } else {
      request.response
        ..statusCode = HttpStatus.notFound
        ..write('VRV Desk Android Host')
        ..close();
    }
  }

  void _handleWebSocketClient(WebSocket socket) {
    final client = _HostClient(socket);
    _clients.add(client);

    // Step 1: Send auth_required to client
    final authReq = jsonEncode({
      'type': 'auth_required',
      'host_name': _deviceName,
      'version': '0.4.0',
    });
    socket.add(authReq);

    client.subscription = socket.listen(
      (data) => _handleClientMessage(client, data),
      onError: (err) {
        debugPrint('[AndroidHostService] Client socket error: $err');
        _removeClient(client);
      },
      onDone: () {
        _removeClient(client);
      },
      cancelOnError: false,
    );
  }

  void _removeClient(_HostClient client) {
    _clients.remove(client);
    client.subscription?.cancel();
    try {
      client.socket.close();
    } catch (_) {}
  }

  Future<void> _handleClientMessage(_HostClient client, dynamic data) async {
    if (data is List<int>) {
      final bytes = Uint8List.fromList(data);
      if (E2eeTransportSession.isE2eePacket(bytes)) {
        if (!client.isAuthenticated || client.e2eeSession == null) {
          debugPrint('[AndroidHostService] Received E2EE packet before auth or without E2EE session');
          return;
        }
        try {
          final decrypted = await client.e2eeSession!.decrypt(bytes);
          final text = utf8.decode(decrypted);
          _handleDecryptedJsonMessage(client, text);
        } catch (e) {
          debugPrint('[AndroidHostService] Decryption error: $e');
        }
      } else {
        // Plain binary message or unencrypted
        try {
          final text = utf8.decode(bytes);
          _handleDecryptedJsonMessage(client, text);
        } catch (_) {}
      }
    } else if (data is String) {
      _handleDecryptedJsonMessage(client, data);
    }
  }

  void _handleDecryptedJsonMessage(_HostClient client, String text) {
    try {
      final json = jsonDecode(text);
      if (json is! Map<String, dynamic>) return;

      final type = json['type']?.toString();

      if (type == 'auth_verify') {
        _handleAuthVerify(client, json);
        return;
      }

      // Input actions require client to be authenticated
      if (!client.isAuthenticated) {
        debugPrint('[AndroidHostService] Ignoring command from unauthenticated client: $type');
        return;
      }

      _dispatchRemoteInput(json);
    } catch (e) {
      debugPrint('[AndroidHostService] JSON parse error: $e');
    }
  }

  void _handleAuthVerify(_HostClient client, Map<String, dynamic> json) {
    final clientPin = json['pin']?.toString().trim() ?? '';
    final clientPassword = json['password']?.toString().trim();
    final candidate = (clientPassword != null && clientPassword.isNotEmpty)
        ? clientPassword
        : clientPin;
    final clientE2ee = json['e2ee'] == true;

    final isPinMatch = candidate == _currentPin.trim();
    final isUnattendedMatch = _unattendedPassword != null &&
        _unattendedPassword!.isNotEmpty &&
        candidate == _unattendedPassword!.trim();

    if (isPinMatch || isUnattendedMatch) {
      client.isAuthenticated = true;
      client.isE2ee = clientE2ee;

      // Generate 16-byte random session token (hex)
      final rand = Random.secure();
      final tokenBytes = List<int>.generate(16, (_) => rand.nextInt(256));
      final sessionToken = tokenBytes.map((b) => b.toRadixString(16).padLeft(2, '0')).join();
      client.sessionToken = sessionToken;

      if (clientE2ee) {
        client.e2eeSession = E2eeTransportSession.fromToken(sessionToken);
      }

      final authOk = jsonEncode({
        'type': 'auth_ok',
        'session_token': sessionToken,
        'e2ee': clientE2ee,
        'unattended': isUnattendedMatch && !isPinMatch,
      });
      client.socket.add(authOk);
    } else {
      client.remainingAttempts = max(0, client.remainingAttempts - 1);
      final hasUnattended = _unattendedPassword != null && _unattendedPassword!.isNotEmpty;
      final authFailed = jsonEncode({
        'type': 'auth_failed',
        'reason': hasUnattended ? 'Invalid PIN or Unattended Password' : 'Invalid PIN',
        'remaining_attempts': client.remainingAttempts,
      });
      client.socket.add(authFailed);

      if (client.remainingAttempts <= 0) {
        _removeClient(client);
      }
    }
  }

  void _dispatchRemoteInput(Map<String, dynamic> json) {
    final type = json['type']?.toString();
    switch (type) {
      case 'touch_tap':
      case 'tap':
      case 'mouse_down':
      case 'touch_down':
        final normX = (json['x'] as num?)?.toDouble() ?? 0.0;
        final normY = (json['y'] as num?)?.toDouble() ?? 0.0;
        final px = (normX * screenWidth).clamp(0.0, screenWidth);
        final py = (normY * screenHeight).clamp(0.0, screenHeight);
        if (useAdbInputBridge) {
          _adbBridgeChannel.invokeMethod('tap', {'x': px, 'y': py});
        } else {
          _accessibilityChannel.invokeMethod('tap', {'x': px, 'y': py});
        }
        break;

      case 'swipe':
        final normX1 = (json['x1'] as num?)?.toDouble() ?? 0.0;
        final normY1 = (json['y1'] as num?)?.toDouble() ?? 0.0;
        final normX2 = (json['x2'] as num?)?.toDouble() ?? 0.0;
        final normY2 = (json['y2'] as num?)?.toDouble() ?? 0.0;
        final duration = (json['duration'] as num?)?.toInt() ?? 300;

        final px1 = (normX1 * screenWidth).clamp(0.0, screenWidth);
        final py1 = (normY1 * screenHeight).clamp(0.0, screenHeight);
        final px2 = (normX2 * screenWidth).clamp(0.0, screenWidth);
        final py2 = (normY2 * screenHeight).clamp(0.0, screenHeight);

        if (useAdbInputBridge) {
          _adbBridgeChannel.invokeMethod('swipe', {
            'x1': px1,
            'y1': py1,
            'x2': px2,
            'y2': py2,
            'duration': duration,
          });
        } else {
          _accessibilityChannel.invokeMethod('swipe', {
            'x1': px1,
            'y1': py1,
            'x2': px2,
            'y2': py2,
            'duration': duration,
          });
        }
        break;

      case 'shortcut':
        final action = json['name']?.toString() ?? json['action']?.toString() ?? '';
        if (action.isNotEmpty) {
          if (useAdbInputBridge) {
            _adbBridgeChannel.invokeMethod('globalAction', {'action': action});
          } else {
            _accessibilityChannel.invokeMethod('globalAction', {'action': action});
          }
        }
        break;

      case 'back':
      case 'home':
      case 'recents':
        if (useAdbInputBridge) {
          _adbBridgeChannel.invokeMethod('globalAction', {'action': type});
        } else {
          _accessibilityChannel.invokeMethod('globalAction', {'action': type});
        }
        break;

      case 'key':
        final keyCode = (json['keyCode'] as num?)?.toInt() ?? 0;
        if (keyCode != 0) {
          _adbBridgeChannel.invokeMethod('keyevent', {'keyCode': keyCode});
        }
        break;

      case 'text':
        final text = json['text']?.toString() ?? '';
        if (text.isNotEmpty) {
          _adbBridgeChannel.invokeMethod('text', {'text': text});
        }
        break;

      case 'clipboard_text':
        final clipText = json['text']?.toString() ?? '';
        if (clipText.isNotEmpty) {
          _lastSentClipboard = clipText;
          Clipboard.setData(ClipboardData(text: clipText));
        }
        break;

      default:
        break;
    }
  }

  void _startClipboardMonitoring() {
    _clipboardTimer?.cancel();
    _clipboardTimer = Timer.periodic(const Duration(milliseconds: 750), (timer) async {
      if (!_isRunning) {
        timer.cancel();
        return;
      }
      final authenticatedClients = _clients.where((c) => c.isAuthenticated).toList();
      if (authenticatedClients.isEmpty) return;

      try {
        final data = await Clipboard.getData(Clipboard.kTextPlain);
        final currentText = data?.text;
        if (currentText != null &&
            currentText.isNotEmpty &&
            currentText != _lastSentClipboard) {
          _lastSentClipboard = currentText;
          _broadcastClipboardSync(currentText);
        }
      } catch (_) {}
    });
  }

  void _broadcastClipboardSync(String text) {
    final payload = jsonEncode({
      'type': 'clipboard_sync',
      'text': text,
    });
    for (final client in _clients.where((c) => c.isAuthenticated)) {
      try {
        client.socket.add(payload);
      } catch (_) {}
    }
  }

  void _startVideoStream() {
    Stream<dynamic>? stream;
    if (_videoStreamOverride != null) {
      stream = _videoStreamOverride;
    } else if (_captureStreamChannel != null) {
      try {
        stream = _captureStreamChannel!.receiveBroadcastStream();
      } catch (e) {
        debugPrint('[AndroidHostService] Error receiving capture stream: $e');
      }
    }

    if (stream != null) {
      _videoSubscription = stream.listen(
        (data) {
          if (data is List<int>) {
            _broadcastFrame(Uint8List.fromList(data));
          }
        },
        onError: (e) {
          debugPrint('[AndroidHostService] Video stream error: $e');
        },
      );
    }
  }

  Future<void> _broadcastFrame(Uint8List frame) async {
    final authenticatedClients = _clients.where((c) => c.isAuthenticated).toList();
    if (authenticatedClients.isEmpty) return;

    for (final client in authenticatedClients) {
      if (client.isE2ee && client.e2eeSession != null) {
        try {
          final encrypted = await client.e2eeSession!.encrypt(frame);
          client.socket.add(encrypted);
        } catch (e) {
          debugPrint('[AndroidHostService] Failed to encrypt frame for client: $e');
          client.socket.add(frame);
        }
      } else {
        client.socket.add(frame);
      }
    }
    _framesSent++;
  }

  /// Generates the payload for the UDP discovery beacon.
  Uint8List generateBeaconPacket({
    required String deviceId,
    required String deviceName,
    required int port,
  }) {
    final map = {
      'device_id': deviceId,
      'device_name': deviceName,
      'os_type': 'android',
      'port': port,
      'protocol_version': 1,
    };
    return Uint8List.fromList(utf8.encode(jsonEncode(map)));
  }

  Future<void> _startUdpBeacon() async {
    try {
      _udpSocket = await RawDatagramSocket.bind(InternetAddress.anyIPv4, 0);
      _udpSocket?.broadcastEnabled = true;

      _beaconTimer?.cancel();
      _beaconTimer = Timer.periodic(const Duration(milliseconds: 1500), (_) {
        if (!_isRunning || _udpSocket == null) return;
        final packet = generateBeaconPacket(
          deviceId: _deviceId,
          deviceName: _deviceName,
          port: _port,
        );
        try {
          _udpSocket?.send(
            packet,
            InternetAddress(defaultMulticastAddress),
            defaultUdpPort,
          );
        } catch (_) {}
        try {
          _udpSocket?.send(
            packet,
            InternetAddress('255.255.255.255'),
            defaultUdpPort,
          );
        } catch (_) {}
      });
    } catch (e) {
      debugPrint('[AndroidHostService] UDP Beacon start error: $e');
    }
  }

  // Helper APIs for Android permission / capture control

  Future<bool> isAccessibilityEnabled() async {
    try {
      final res = await _accessibilityChannel.invokeMethod<bool>('isAccessibilityEnabled');
      return res ?? false;
    } catch (_) {
      return false;
    }
  }

  Future<bool> openAccessibilitySettings() async {
    try {
      final res = await _accessibilityChannel.invokeMethod<bool>('openAccessibilitySettings');
      return res ?? false;
    } catch (_) {
      return false;
    }
  }

  Future<bool> isInternalAudioSupported() async {
    try {
      final res = await _captureChannel.invokeMethod<bool>('isInternalAudioSupported');
      return res ?? false;
    } catch (_) {
      return false;
    }
  }

  Future<bool> startCapture({
    int resultCode = -1, // Activity.RESULT_OK
    dynamic intentData,
    int width = 1280,
    int height = 720,
    int bitrate = 2500000,
    int fps = 30,
    bool enableAudio = true,
  }) async {
    try {
      final res = await _captureChannel.invokeMethod<bool>('startCapture', {
        'resultCode': resultCode,
        'intentData': intentData,
        'width': width,
        'height': height,
        'bitrate': bitrate,
        'fps': fps,
        'enableAudio': enableAudio,
      });
      return res ?? false;
    } catch (_) {
      return false;
    }
  }

  Future<bool> stopCapture() async {
    try {
      final res = await _captureChannel.invokeMethod<bool>('stopCapture');
      return res ?? false;
    } catch (_) {
      return false;
    }
  }

  Future<bool> isCapturing() async {
    try {
      final res = await _captureChannel.invokeMethod<bool>('isCapturing');
      return res ?? false;
    } catch (_) {
      return false;
    }
  }

  // High-Performance ADB Bridge APIs

  Future<Map<String, dynamic>?> getAdbStatus() async {
    try {
      final res = await _adbBridgeChannel.invokeMethod<Map<dynamic, dynamic>>('getAdbStatus');
      if (res == null) return null;
      return res.map((k, v) => MapEntry(k.toString(), v));
    } catch (_) {
      return null;
    }
  }

  Future<bool> isAdbHighPerformanceAvailable() async {
    try {
      final status = await getAdbStatus();
      return status?['high_performance_available'] == true;
    } catch (_) {
      return false;
    }
  }

  Future<double> getMaxDisplayRefreshRate() async {
    try {
      final status = await getAdbStatus();
      final rate = (status?['max_refresh_rate'] as num?)?.toDouble();
      return rate ?? 60.0;
    } catch (_) {
      return 60.0;
    }
  }
}
