import 'dart:async';
import 'dart:convert';
import 'dart:io';

class RemoteFileEntry {
  final String name;
  final bool isDir;
  final int size;
  final int modifiedMs;

  RemoteFileEntry({
    required this.name,
    required this.isDir,
    required this.size,
    required this.modifiedMs,
  });

  factory RemoteFileEntry.fromJson(Map<String, dynamic> json) {
    return RemoteFileEntry(
      name: json['name'] as String? ?? '',
      isDir: json['is_dir'] as bool? ?? false,
      size: (json['size'] as num?)?.toInt() ?? 0,
      modifiedMs: (json['modified_ms'] as num?)?.toInt() ?? 0,
    );
  }

  Map<String, dynamic> toJson() => {
        'name': name,
        'is_dir': isDir,
        'size': size,
        'modified_ms': modifiedMs,
      };

  String get formattedSize {
    if (isDir) return '<DIR>';
    if (size < 1024) return '$size B';
    if (size < 1024 * 1024) return '${(size / 1024).toStringAsFixed(1)} KB';
    if (size < 1024 * 1024 * 1024) return '${(size / (1024 * 1024)).toStringAsFixed(1)} MB';
    return '${(size / (1024 * 1024 * 1024)).toStringAsFixed(2)} GB';
  }
}

class FileTransferService {
  final Stream<dynamic> incomingStream;
  final void Function(String message) sendMessage;

  final Map<String, Completer<Map<String, dynamic>>> _pendingRequests = {};
  int _reqCounter = 0;
  StreamSubscription? _subscription;

  FileTransferService({
    required this.incomingStream,
    required this.sendMessage,
  }) {
    _subscription = incomingStream.listen(_handleIncomingMessage);
  }

  void dispose() {
    _subscription?.cancel();
    for (final c in _pendingRequests.values) {
      if (!c.isCompleted) {
        c.completeError('Service disposed');
      }
    }
    _pendingRequests.clear();
  }

  void _handleIncomingMessage(dynamic data) {
    if (data is! String) return;
    try {
      final json = jsonDecode(data);
      if (json is Map<String, dynamic>) {
        final id = json['id'] as String?;
        if (id != null && _pendingRequests.containsKey(id)) {
          _pendingRequests.remove(id)!.complete(json);
        }
      }
    } catch (_) {}
  }

  String _nextId() => 'fs_${++_reqCounter}_${DateTime.now().millisecondsSinceEpoch}';

  /// Lists a remote directory on the host.
  Future<({String path, List<RemoteFileEntry> entries})> listRemoteDirectory(String path) async {
    final id = _nextId();
    final completer = Completer<Map<String, dynamic>>();
    _pendingRequests[id] = completer;

    sendMessage(jsonEncode({
      'type': 'fs_list',
      'id': id,
      'path': path,
    }));

    final resp = await completer.future.timeout(const Duration(seconds: 15));
    if (resp['type'] == 'fs_error') {
      throw Exception(resp['error'] ?? 'Failed to list directory');
    }

    final canonicalPath = resp['path'] as String? ?? path;
    final rawEntries = resp['entries'] as List<dynamic>? ?? [];
    final entries = rawEntries
        .map((e) => RemoteFileEntry.fromJson(e as Map<String, dynamic>))
        .toList();

    return (path: canonicalPath, entries: entries);
  }

  /// Downloads a remote file in 64KB chunks to local destination.
  Future<void> downloadFile(
    String remotePath,
    String localPath, {
    void Function(double progress, int bytesReceived, int totalBytes)? onProgress,
  }) async {
    final localFile = File(localPath);
    if (!localFile.parent.existsSync()) {
      localFile.parent.createSync(recursive: true);
    }

    final sink = localFile.openWrite();
    int offset = 0;
    int totalSize = 1;
    bool eof = false;

    try {
      while (!eof) {
        final id = _nextId();
        final completer = Completer<Map<String, dynamic>>();
        _pendingRequests[id] = completer;

        sendMessage(jsonEncode({
          'type': 'fs_read_chunk',
          'id': id,
          'path': remotePath,
          'offset': offset,
          'length': 65536,
        }));

        final resp = await completer.future.timeout(const Duration(seconds: 30));
        if (resp['type'] == 'fs_error') {
          throw Exception(resp['error'] ?? 'Read failed');
        }

        totalSize = (resp['total_size'] as num?)?.toInt() ?? 1;
        eof = resp['eof'] as bool? ?? true;
        final dataB64 = resp['data_b64'] as String? ?? '';
        final bytes = base64Decode(dataB64);

        if (bytes.isNotEmpty) {
          sink.add(bytes);
          offset += bytes.length;
        } else if (eof) {
          break;
        }

        final ratio = (offset / (totalSize > 0 ? totalSize : 1)).clamp(0.0, 1.0);
        onProgress?.call(ratio, offset, totalSize);
      }
      await sink.flush();
    } finally {
      await sink.close();
    }
  }

  /// Uploads a local file in 64KB chunks to remote host destination.
  Future<void> uploadFile(
    String localPath,
    String remotePath, {
    void Function(double progress, int bytesSent, int totalBytes)? onProgress,
  }) async {
    final localFile = File(localPath);
    if (!localFile.existsSync()) {
      throw Exception('Local file does not exist: $localPath');
    }

    final totalSize = localFile.lengthSync();
    final raf = localFile.openSync(mode: FileMode.read);
    int offset = 0;

    try {
      if (totalSize == 0) {
        // Upload empty file
        final id = _nextId();
        final completer = Completer<Map<String, dynamic>>();
        _pendingRequests[id] = completer;

        sendMessage(jsonEncode({
          'type': 'fs_write_chunk',
          'id': id,
          'path': remotePath,
          'offset': 0,
          'data_b64': '',
          'eof': true,
        }));

        await completer.future.timeout(const Duration(seconds: 15));
        onProgress?.call(1.0, 0, 0);
        return;
      }

      while (offset < totalSize) {
        final chunkSize = (totalSize - offset) > 65536 ? 65536 : (totalSize - offset);
        final bytes = raf.readSync(chunkSize);
        final eof = (offset + bytes.length) >= totalSize;
        final dataB64 = base64Encode(bytes);

        final id = _nextId();
        final completer = Completer<Map<String, dynamic>>();
        _pendingRequests[id] = completer;

        sendMessage(jsonEncode({
          'type': 'fs_write_chunk',
          'id': id,
          'path': remotePath,
          'offset': offset,
          'data_b64': dataB64,
          'eof': eof,
        }));

        final resp = await completer.future.timeout(const Duration(seconds: 30));
        if (resp['type'] == 'fs_error' || resp['success'] != true) {
          throw Exception(resp['error'] ?? 'Write chunk failed');
        }

        offset += bytes.length;
        final ratio = (offset / totalSize).clamp(0.0, 1.0);
        onProgress?.call(ratio, offset, totalSize);
      }
    } finally {
      raf.closeSync();
    }
  }

  /// Creates a directory on remote host.
  Future<bool> createRemoteDirectory(String path) async {
    final id = _nextId();
    final completer = Completer<Map<String, dynamic>>();
    _pendingRequests[id] = completer;

    sendMessage(jsonEncode({
      'type': 'fs_mkdir',
      'id': id,
      'path': path,
    }));

    final resp = await completer.future.timeout(const Duration(seconds: 15));
    if (resp['type'] == 'fs_error') {
      throw Exception(resp['error'] ?? 'Failed to create directory');
    }
    return resp['success'] == true;
  }

  /// Deletes a file or directory on remote host.
  Future<bool> deleteRemoteItem(String path, bool isDir) async {
    final id = _nextId();
    final completer = Completer<Map<String, dynamic>>();
    _pendingRequests[id] = completer;

    sendMessage(jsonEncode({
      'type': 'fs_delete',
      'id': id,
      'path': path,
      'is_dir': isDir,
    }));

    final resp = await completer.future.timeout(const Duration(seconds: 15));
    if (resp['type'] == 'fs_error') {
      throw Exception(resp['error'] ?? 'Failed to delete item');
    }
    return resp['success'] == true;
  }

  /// Lists local files in a directory.
  static Future<({String path, List<RemoteFileEntry> entries})> listLocalDirectory(String rawPath) async {
    String currentPath = rawPath;
    if (currentPath.isEmpty || currentPath == 'roots') {
      currentPath = Directory.current.path;
    }

    final dir = Directory(currentPath);
    if (!dir.existsSync()) {
      throw Exception('Local directory does not exist: $currentPath');
    }

    final List<RemoteFileEntry> entries = [];
    final entities = dir.listSync();

    for (final entity in entities) {
      final name = entity.uri.pathSegments.where((s) => s.isNotEmpty).last;
      final isDir = entity is Directory;
      int size = 0;
      int modifiedMs = 0;
      try {
        final stat = entity.statSync();
        size = isDir ? 0 : stat.size;
        modifiedMs = stat.modified.millisecondsSinceEpoch;
      } catch (_) {}

      entries.push(RemoteFileEntry(
        name: name,
        isDir: isDir,
        size: size,
        modifiedMs: modifiedMs,
      ));
    }

    entries.sort((a, b) {
      if (a.isDir != b.isDir) {
        return b.isDir ? 1 : -1;
      }
      return a.name.toLowerCase().compareTo(b.name.toLowerCase());
    });

    return (path: dir.path, entries: entries);
  }
}

extension _ListPush<T> on List<T> {
  void push(T item) => add(item);
}
