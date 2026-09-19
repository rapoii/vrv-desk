import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'package:flutter/foundation.dart';
import '../models/device.dart';

class LanDiscoveryService {
  static const String multicastAddress = '239.255.42.99';
  static const int discoveryPort = 53210;
  static const Duration staleTimeout = Duration(seconds: 6);
  static const Duration pruneInterval = Duration(seconds: 2);

  RawDatagramSocket? _socket;
  final Map<String, DiscoveredDevice> _devices = {};
  final StreamController<List<DiscoveredDevice>> _controller =
      StreamController<List<DiscoveredDevice>>.broadcast();
  Timer? _pruneTimer;
  bool _isStarted = false;

  Stream<List<DiscoveredDevice>> get devicesStream => _controller.stream;
  List<DiscoveredDevice> get currentDevices => _devices.values.toList();
  bool get isRunning => _isStarted;

  Future<void> start() async {
    if (_isStarted) return;
    _isStarted = true;

    try {
      // Bind to discoveryPort.
      // On Windows, reusePort is not supported and logs an OS error to stderr.
      // On Android / Linux / macOS, reusePort allows multiple sockets to bind to the same multicast port.
      final useReusePort = !Platform.isWindows;
      try {
        _socket = await RawDatagramSocket.bind(
          InternetAddress.anyIPv4,
          discoveryPort,
          reuseAddress: true,
          reusePort: useReusePort,
        );
      } catch (e) {
        // Fallback without reusePort if OS throws an unsupported socket option error
        _socket = await RawDatagramSocket.bind(
          InternetAddress.anyIPv4,
          discoveryPort,
          reuseAddress: true,
          reusePort: false,
        );
      }

      // Join multicast group
      try {
        _socket?.joinMulticast(InternetAddress(multicastAddress));
      } catch (e) {
        debugPrint('[LanDiscoveryService] Warning: joinMulticast failed: $e');
      }

      // Listen for UDP packets
      _socket?.listen(
        (RawSocketEvent event) {
          if (event == RawSocketEvent.read) {
            final datagram = _socket?.receive();
            if (datagram != null) {
              final senderIp = _resolveSenderIp(datagram.address);
              handleBeaconData(datagram.data, senderIp);
            }
          }
        },
        onError: (e) {
          debugPrint('[LanDiscoveryService] Socket error: $e');
        },
        cancelOnError: false,
      );
    } catch (e) {
      debugPrint('[LanDiscoveryService] Failed to bind UDP discovery socket: $e');
    }

    // Start periodic pruning of stale devices
    _pruneTimer?.cancel();
    _pruneTimer = Timer.periodic(pruneInterval, (_) => pruneStaleDevices());
  }

  String _resolveSenderIp(InternetAddress address) {
    // If address is loopback or 0.0.0.0, keep it or normalize if needed
    final ip = address.address;
    if (ip == '0.0.0.0') {
      return '127.0.0.1';
    }
    return ip;
  }

  /// Handles and parses raw beacon bytes. Exposed for unit testing without sockets.
  void handleBeaconData(Uint8List data, String senderIp) {
    try {
      final jsonStr = utf8.decode(data);
      final dynamic decoded = jsonDecode(jsonStr);
      if (decoded is! Map<String, dynamic>) {
        return;
      }

      final deviceId = decoded['device_id']?.toString() ?? '';
      final deviceName = decoded['device_name']?.toString() ?? 'Unknown Device';
      final osType = decoded['os_type']?.toString() ?? 'windows';
      final port = decoded['port'] is int ? decoded['port'] as int : 53211;

      if (deviceId.isEmpty) {
        return;
      }

      final explicitIp = decoded['host_ip']?.toString() ?? decoded['ip']?.toString();
      final effectiveIp = (explicitIp != null && explicitIp.isNotEmpty)
          ? explicitIp
          : (senderIp == '127.0.0.1' ? '10.0.2.2' : senderIp);

      final device = DiscoveredDevice(
        deviceId: deviceId,
        deviceName: deviceName,
        osType: osType,
        ipAddress: effectiveIp,
        port: port,
        lastSeen: DateTime.now(),
      );

      _devices[deviceId] = device;
      _emitDevices();
    } catch (e) {
      debugPrint('[LanDiscoveryService] Malformed beacon packet: $e');
    }
  }

  /// Checks for and removes stale devices that haven't sent a beacon within staleTimeout.
  void pruneStaleDevices({DateTime? nowOverride}) {
    final now = nowOverride ?? DateTime.now();
    bool changed = false;

    _devices.removeWhere((id, device) {
      final isStale = now.difference(device.lastSeen) > staleTimeout;
      if (isStale) {
        changed = true;
      }
      return isStale;
    });

    if (changed) {
      _emitDevices();
    }
  }

  void _emitDevices() {
    if (!_controller.isClosed) {
      _controller.add(_devices.values.toList());
    }
  }

  Future<void> stop() async {
    _isStarted = false;
    _pruneTimer?.cancel();
    _pruneTimer = null;

    try {
      _socket?.close();
    } catch (_) {}
    _socket = null;

    if (!_controller.isClosed) {
      await _controller.close();
    }
  }
}
