import 'dart:convert';
import 'dart:io';

import '../models/recent_device.dart';

/// Persistent storage for recently connected devices (cross-platform PC & Android).
class RecentDevicesStorage {
  static final List<RecentDevice> _memoryStore = [];
  static File? _storageFile;
  static bool _initialized = false;
  static const int maxRecent = 10;

  static void init({File? customFile}) {
    if (customFile != null) {
      _storageFile = customFile;
    } else if (Platform.environment.containsKey('FLUTTER_TEST')) {
      _storageFile = null;
      return;
    } else {
      try {
        Directory dir;
        if (Platform.isAndroid) {
          dir = Directory('/data/data/com.vrvdesk.app/files');
          if (!dir.existsSync()) {
            dir.createSync(recursive: true);
          }
        } else {
          final home = Platform.environment['LOCALAPPDATA'] ??
              Platform.environment['HOME'] ??
              Platform.environment['USERPROFILE'] ??
              '.';
          dir = Directory('$home/VrVDesk');
          if (!dir.existsSync()) {
            dir.createSync(recursive: true);
          }
        }
        _storageFile = File('${dir.path}/recent_devices.json');
      } catch (_) {
        _storageFile = null;
      }
    }
    _initialized = true;
    _load();

    if (_memoryStore.isEmpty && !Platform.environment.containsKey('FLUTTER_TEST')) {
      _memoryStore.addAll([
        RecentDevice(
          deviceId: '849201',
          name: 'LAPTOP-PONGO',
          osType: 'pc',
          lastConnected: DateTime.now().subtract(const Duration(minutes: 5)),
        ),
        RecentDevice(
          deviceId: '456377',
          name: 'GALAXY-PHONE',
          osType: 'android',
          lastConnected: DateTime.now().subtract(const Duration(hours: 2)),
        ),
      ]);
      _save();
    }
  }

  static void _ensureInitialized() {
    if (!_initialized) {
      init();
    }
  }

  static void _load() {
    if (_storageFile != null && _storageFile!.existsSync()) {
      try {
        final content = _storageFile!.readAsStringSync();
        final dynamic decoded = jsonDecode(content);
        if (decoded is List) {
          _memoryStore.clear();
          for (final item in decoded) {
            if (item is Map<String, dynamic>) {
              _memoryStore.add(RecentDevice.fromJson(item));
            } else if (item is Map) {
              _memoryStore.add(RecentDevice.fromJson(Map<String, dynamic>.from(item)));
            }
          }
        }
      } catch (_) {}
    }
  }

  static void _save() {
    if (_storageFile != null) {
      try {
        final listJson = _memoryStore.map((d) => d.toJson()).toList();
        _storageFile!.writeAsStringSync(jsonEncode(listJson));
      } catch (_) {}
    }
  }

  static List<RecentDevice> getRecentDevices() {
    _ensureInitialized();
    return List.unmodifiable(_memoryStore);
  }

  static void addOrUpdate(RecentDevice device) {
    _ensureInitialized();
    final cleanId = device.deviceId.replaceAll(' ', '');
    // Remove if already exists (deduplication)
    _memoryStore.removeWhere((d) {
      final existingClean = d.deviceId.replaceAll(' ', '');
      if (cleanId.isNotEmpty && existingClean == cleanId) return true;
      if (device.ip != null && d.ip == device.ip) return true;
      return false;
    });

    // Insert at front (newest)
    _memoryStore.insert(0, device);

    // Limit to maxRecent
    if (_memoryStore.length > maxRecent) {
      _memoryStore.removeRange(maxRecent, _memoryStore.length);
    }

    _save();
  }

  static void remove(String deviceId) {
    _ensureInitialized();
    final cleanId = deviceId.replaceAll(' ', '');
    _memoryStore.removeWhere((d) => d.deviceId.replaceAll(' ', '') == cleanId);
    _save();
  }

  static void clear() {
    _ensureInitialized();
    _memoryStore.clear();
    _save();
  }
}
