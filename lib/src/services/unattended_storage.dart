import 'dart:convert';
import 'dart:io';

/// Lightweight persistent storage for remembered Unattended Access passwords.
class UnattendedStorage {
  static final Map<String, String> _memoryStore = {};
  static File? _storageFile;
  static bool _initialized = false;

  static void init({File? customFile}) {
    if (customFile != null) {
      _storageFile = customFile;
    } else if (Platform.environment.containsKey('FLUTTER_TEST')) {
      // In Flutter test environment, keep purely in-memory to prevent test pollution
      _storageFile = null;
    } else {
      try {
        final home = Platform.environment['LOCALAPPDATA'] ??
            Platform.environment['HOME'] ??
            Platform.environment['USERPROFILE'] ??
            '.';
        final dir = Directory('$home/VrVDesk');
        if (!dir.existsSync()) {
          dir.createSync(recursive: true);
        }
        _storageFile = File('${dir.path}/saved_unattended.json');
      } catch (_) {
        _storageFile = null;
      }
    }
    _initialized = true;
    _load();
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
        if (decoded is Map) {
          _memoryStore.clear();
          decoded.forEach((k, v) {
            if (v is String) _memoryStore[k.toString()] = v;
          });
        }
      } catch (_) {}
    }
  }

  static void _save() {
    if (_storageFile != null) {
      try {
        _storageFile!.writeAsStringSync(jsonEncode(_memoryStore));
      } catch (_) {}
    }
  }

  static String? getSavedPassword(String deviceIdOrIp) {
    _ensureInitialized();
    final clean = deviceIdOrIp.replaceAll(' ', '');
    return _memoryStore[clean];
  }

  static void savePassword(String deviceIdOrIp, String password) {
    _ensureInitialized();
    final clean = deviceIdOrIp.replaceAll(' ', '');
    _memoryStore[clean] = password;
    _save();
  }

  static void removePassword(String deviceIdOrIp) {
    _ensureInitialized();
    final clean = deviceIdOrIp.replaceAll(' ', '');
    _memoryStore.remove(clean);
    _save();
  }

  static bool hasSavedPassword(String deviceIdOrIp) {
    _ensureInitialized();
    final clean = deviceIdOrIp.replaceAll(' ', '');
    return _memoryStore.containsKey(clean) && _memoryStore[clean]!.isNotEmpty;
  }

  static void clearAll() {
    _memoryStore.clear();
    _save();
  }
}
