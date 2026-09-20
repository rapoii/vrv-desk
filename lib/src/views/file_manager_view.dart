import 'dart:io';

import 'package:flutter/material.dart';

import '../services/file_transfer_service.dart';
import '../theme/neobrutalist_theme.dart';

class FileManagerView extends StatefulWidget {
  final FileTransferService service;
  final String remoteHostName;
  final String initialLocalPath;
  final String initialRemotePath;

  const FileManagerView({
    super.key,
    required this.service,
    this.remoteHostName = 'Remote Host',
    this.initialLocalPath = '',
    this.initialRemotePath = '',
  });

  @override
  State<FileManagerView> createState() => _FileManagerViewState();
}

class _FileManagerViewState extends State<FileManagerView>
    with SingleTickerProviderStateMixin {
  late TabController _tabController;

  String _localPath = '';
  List<RemoteFileEntry> _localEntries = [];
  RemoteFileEntry? _selectedLocalEntry;
  bool _isLoadingLocal = false;

  String _remotePath = '';
  List<RemoteFileEntry> _remoteEntries = [];
  RemoteFileEntry? _selectedRemoteEntry;
  bool _isLoadingRemote = false;

  // Transfer state
  bool _isTransferring = false;
  String _transferTitle = '';
  double _transferProgress = 0.0;
  String _transferDetail = '';

  @override
  void initState() {
    super.initState();
    _tabController = TabController(length: 2, vsync: this);
    _localPath = widget.initialLocalPath.isNotEmpty
        ? widget.initialLocalPath
        : Directory.current.path;
    _remotePath = widget.initialRemotePath;

    _loadLocalDirectory(_localPath);
    _loadRemoteDirectory(_remotePath);
  }

  @override
  void dispose() {
    _tabController.dispose();
    super.dispose();
  }

  Future<void> _loadLocalDirectory(String path) async {
    setState(() {
      _isLoadingLocal = true;
      _selectedLocalEntry = null;
    });
    try {
      final res = await FileTransferService.listLocalDirectory(path);
      setState(() {
        _localPath = res.path;
        _localEntries = res.entries;
        _isLoadingLocal = false;
      });
    } catch (e) {
      setState(() => _isLoadingLocal = false);
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
            content: Text('Error reading local dir: $e'),
            backgroundColor: Colors.red,
          ),
        );
      }
    }
  }

  Future<void> _loadRemoteDirectory(String path) async {
    setState(() {
      _isLoadingRemote = true;
      _selectedRemoteEntry = null;
    });
    try {
      final res = await widget.service.listRemoteDirectory(path);
      setState(() {
        _remotePath = res.path;
        _remoteEntries = res.entries;
        _isLoadingRemote = false;
      });
    } catch (e) {
      setState(() => _isLoadingRemote = false);
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
            content: Text('Error reading remote dir: $e'),
            backgroundColor: Colors.red,
          ),
        );
      }
    }
  }

  void _navigateLocalUp() {
    final parent = Directory(_localPath).parent.path;
    if (parent != _localPath) {
      _loadLocalDirectory(parent);
    }
  }

  void _navigateRemoteUp() {
    if (_remotePath == 'Roots' || _remotePath.isEmpty) return;

    // Standard Windows / POSIX up
    final separator = _remotePath.contains('\\') ? '\\' : '/';
    final parts = _remotePath
        .split(separator)
        .where((s) => s.isNotEmpty)
        .toList();
    if (parts.length <= 1) {
      _loadRemoteDirectory('roots');
    } else {
      parts.removeLast();
      final upPath =
          parts.join(separator) +
          (parts.length == 1 && _remotePath.contains(':') ? separator : '');
      _loadRemoteDirectory(upPath);
    }
  }

  Future<void> _startDownload() async {
    final remoteItem = _selectedRemoteEntry;
    if (remoteItem == null || remoteItem.isDir) return;

    final sep = Platform.isWindows ? '\\' : '/';
    final localDest = '$_localPath$sep${remoteItem.name}';
    final remoteSep = _remotePath.contains('\\') ? '\\' : '/';
    final remoteSrc = '$_remotePath$remoteSep${remoteItem.name}';

    setState(() {
      _isTransferring = true;
      _transferTitle = 'Downloading ${remoteItem.name}';
      _transferProgress = 0.0;
      _transferDetail = 'Starting download...';
    });

    try {
      await widget.service.downloadFile(
        remoteSrc,
        localDest,
        onProgress: (progress, received, total) {
          if (mounted) {
            setState(() {
              _transferProgress = progress;
              _transferDetail =
                  '${(progress * 100).toStringAsFixed(1)}% ($received / $total B)';
            });
          }
        },
      );
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
            content: Text('Downloaded: ${remoteItem.name}'),
            backgroundColor: Colors.green,
          ),
        );
      }
      _loadLocalDirectory(_localPath);
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
            content: Text('Download failed: $e'),
            backgroundColor: Colors.red,
          ),
        );
      }
    } finally {
      if (mounted) setState(() => _isTransferring = false);
    }
  }

  Future<void> _startUpload() async {
    final localItem = _selectedLocalEntry;
    if (localItem == null || localItem.isDir) return;

    final sep = Platform.isWindows ? '\\' : '/';
    final localSrc = '$_localPath$sep${localItem.name}';
    final remoteSep = _remotePath.contains('\\') ? '\\' : '/';
    final remoteDest = '$_remotePath$remoteSep${localItem.name}';

    setState(() {
      _isTransferring = true;
      _transferTitle = 'Uploading ${localItem.name}';
      _transferProgress = 0.0;
      _transferDetail = 'Starting upload...';
    });

    try {
      await widget.service.uploadFile(
        localSrc,
        remoteDest,
        onProgress: (progress, sent, total) {
          if (mounted) {
            setState(() {
              _transferProgress = progress;
              _transferDetail =
                  '${(progress * 100).toStringAsFixed(1)}% ($sent / $total B)';
            });
          }
        },
      );
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
            content: Text('Uploaded: ${localItem.name}'),
            backgroundColor: Colors.green,
          ),
        );
      }
      _loadRemoteDirectory(_remotePath);
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
            content: Text('Upload failed: $e'),
            backgroundColor: Colors.red,
          ),
        );
      }
    } finally {
      if (mounted) setState(() => _isTransferring = false);
    }
  }

  Future<void> _createNewFolderDialog(bool isRemote) async {
    final controller = TextEditingController();
    final name = await showDialog<String>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: Text(isRemote ? 'New Remote Folder' : 'New Local Folder'),
        content: TextField(
          controller: controller,
          decoration: const InputDecoration(hintText: 'Folder name'),
          autofocus: true,
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx),
            child: const Text('Cancel'),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(ctx, controller.text.trim()),
            child: const Text('Create'),
          ),
        ],
      ),
    );

    if (name == null || name.isEmpty) return;

    if (isRemote) {
      final sep = _remotePath.contains('\\') ? '\\' : '/';
      final path = '$_remotePath$sep$name';
      try {
        await widget.service.createRemoteDirectory(path);
        _loadRemoteDirectory(_remotePath);
      } catch (e) {
        if (mounted) {
          ScaffoldMessenger.of(context).showSnackBar(
            SnackBar(
              content: Text('Create failed: $e'),
              backgroundColor: Colors.red,
            ),
          );
        }
      }
    } else {
      final sep = Platform.isWindows ? '\\' : '/';
      final path = '$_localPath$sep$name';
      try {
        Directory(path).createSync(recursive: true);
        _loadLocalDirectory(_localPath);
      } catch (e) {
        if (mounted) {
          ScaffoldMessenger.of(context).showSnackBar(
            SnackBar(
              content: Text('Create failed: $e'),
              backgroundColor: Colors.red,
            ),
          );
        }
      }
    }
  }

  Future<void> _deleteDialog(bool isRemote) async {
    final item = isRemote ? _selectedRemoteEntry : _selectedLocalEntry;
    if (item == null) return;

    final confirm = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: Text('Delete ${item.isDir ? "Folder" : "File"}?'),
        content: Text(
          'Are you sure you want to permanently delete "${item.name}"?',
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(ctx, false),
            child: const Text('Cancel'),
          ),
          FilledButton(
            style: FilledButton.styleFrom(
              backgroundColor: NeobrutalTheme.danger,
              foregroundColor: NeobrutalTheme.ink,
            ),
            onPressed: () => Navigator.pop(ctx, true),
            child: const Text('Delete'),
          ),
        ],
      ),
    );

    if (confirm != true) return;

    if (isRemote) {
      final sep = _remotePath.contains('\\') ? '\\' : '/';
      final path = '$_remotePath$sep${item.name}';
      try {
        await widget.service.deleteRemoteItem(path, item.isDir);
        _loadRemoteDirectory(_remotePath);
      } catch (e) {
        if (mounted) {
          ScaffoldMessenger.of(context).showSnackBar(
            SnackBar(
              content: Text('Delete failed: $e'),
              backgroundColor: Colors.red,
            ),
          );
        }
      }
    } else {
      final sep = Platform.isWindows ? '\\' : '/';
      final path = '$_localPath$sep${item.name}';
      try {
        if (item.isDir) {
          Directory(path).deleteSync(recursive: true);
        } else {
          File(path).deleteSync();
        }
        _loadLocalDirectory(_localPath);
      } catch (e) {
        if (mounted) {
          ScaffoldMessenger.of(context).showSnackBar(
            SnackBar(
              content: Text('Delete failed: $e'),
              backgroundColor: Colors.red,
            ),
          );
        }
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final isDesktop = MediaQuery.of(context).size.width >= 720;

    return Scaffold(
      appBar: AppBar(
        title: Text('File Manager • ${widget.remoteHostName}'),
        actions: [
          IconButton(
            icon: const Icon(Icons.refresh),
            tooltip: 'Refresh both',
            onPressed: () {
              _loadLocalDirectory(_localPath);
              _loadRemoteDirectory(_remotePath);
            },
          ),
        ],
        bottom: isDesktop
            ? null
            : TabBar(
                controller: _tabController,
                tabs: const [
                  Tab(icon: Icon(Icons.phone_android), text: 'Local Files'),
                  Tab(icon: Icon(Icons.computer), text: 'Remote Files'),
                ],
              ),
      ),
      body: Stack(
        children: [
          Column(
            children: [
              // Middle Action Transfer Bar
              Container(
                decoration: const BoxDecoration(
                  color: NeobrutalTheme.yellow,
                  border: Border(
                    bottom: BorderSide(
                      color: NeobrutalTheme.ink,
                      width: NeobrutalTheme.borderWidth,
                    ),
                  ),
                ),
                padding: const EdgeInsets.symmetric(
                  horizontal: 12,
                  vertical: 8,
                ),
                child: SingleChildScrollView(
                  scrollDirection: Axis.horizontal,
                  child: Row(
                    children: [
                      ElevatedButton.icon(
                        onPressed:
                            (_selectedRemoteEntry != null &&
                                !_selectedRemoteEntry!.isDir)
                            ? _startDownload
                            : null,
                        icon: const Icon(Icons.download, size: 18),
                        label: const Text('Download to Local'),
                      ),
                      const SizedBox(width: 8),
                      ElevatedButton.icon(
                        onPressed:
                            (_selectedLocalEntry != null &&
                                !_selectedLocalEntry!.isDir)
                            ? _startUpload
                            : null,
                        icon: const Icon(Icons.upload, size: 18),
                        label: const Text('Upload to Remote'),
                      ),
                      const SizedBox(width: 16),
                      Text(
                        _selectedRemoteEntry != null
                            ? 'Remote: ${_selectedRemoteEntry!.name}'
                            : (_selectedLocalEntry != null
                                  ? 'Local: ${_selectedLocalEntry!.name}'
                                  : ''),
                        style: const TextStyle(
                          fontSize: 12,
                          fontWeight: FontWeight.w800,
                          color: NeobrutalTheme.ink,
                        ),
                        overflow: TextOverflow.ellipsis,
                      ),
                    ],
                  ),
                ),
              ),

              // Main Panels
              Expanded(
                child: isDesktop
                    ? Row(
                        children: [
                          Expanded(
                            child: _buildPane(
                              title: 'Local Device',
                              icon: Icons.phone_android,
                              currentPath: _localPath,
                              entries: _localEntries,
                              selectedEntry: _selectedLocalEntry,
                              isLoading: _isLoadingLocal,
                              onNavigateUp: _navigateLocalUp,
                              onEntrySelected: (e) =>
                                  setState(() => _selectedLocalEntry = e),
                              onEntryDoubleTap: (e) {
                                if (e.isDir) {
                                  final sep = Platform.isWindows ? '\\' : '/';
                                  _loadLocalDirectory(
                                    '$_localPath$sep${e.name}',
                                  );
                                }
                              },
                              onCreateFolder: () =>
                                  _createNewFolderDialog(false),
                              onDeleteItem: () => _deleteDialog(false),
                            ),
                          ),
                          const VerticalDivider(width: 1),
                          Expanded(
                            child: _buildPane(
                              title: 'Remote Host (${widget.remoteHostName})',
                              icon: Icons.computer,
                              currentPath: _remotePath,
                              entries: _remoteEntries,
                              selectedEntry: _selectedRemoteEntry,
                              isLoading: _isLoadingRemote,
                              onNavigateUp: _navigateRemoteUp,
                              onEntrySelected: (e) =>
                                  setState(() => _selectedRemoteEntry = e),
                              onEntryDoubleTap: (e) {
                                if (e.isDir) {
                                  final sep = _remotePath.contains('\\')
                                      ? '\\'
                                      : '/';
                                  _loadRemoteDirectory(
                                    '$_remotePath$sep${e.name}',
                                  );
                                }
                              },
                              onCreateFolder: () =>
                                  _createNewFolderDialog(true),
                              onDeleteItem: () => _deleteDialog(true),
                            ),
                          ),
                        ],
                      )
                    : TabBarView(
                        controller: _tabController,
                        children: [
                          _buildPane(
                            title: 'Local Device',
                            icon: Icons.phone_android,
                            currentPath: _localPath,
                            entries: _localEntries,
                            selectedEntry: _selectedLocalEntry,
                            isLoading: _isLoadingLocal,
                            onNavigateUp: _navigateLocalUp,
                            onEntrySelected: (e) =>
                                setState(() => _selectedLocalEntry = e),
                            onEntryDoubleTap: (e) {
                              if (e.isDir) {
                                final sep = Platform.isWindows ? '\\' : '/';
                                _loadLocalDirectory('$_localPath$sep${e.name}');
                              }
                            },
                            onCreateFolder: () => _createNewFolderDialog(false),
                            onDeleteItem: () => _deleteDialog(false),
                          ),
                          _buildPane(
                            title: 'Remote Host',
                            icon: Icons.computer,
                            currentPath: _remotePath,
                            entries: _remoteEntries,
                            selectedEntry: _selectedRemoteEntry,
                            isLoading: _isLoadingRemote,
                            onNavigateUp: _navigateRemoteUp,
                            onEntrySelected: (e) =>
                                setState(() => _selectedRemoteEntry = e),
                            onEntryDoubleTap: (e) {
                              if (e.isDir) {
                                final sep = _remotePath.contains('\\')
                                    ? '\\'
                                    : '/';
                                _loadRemoteDirectory(
                                  '$_remotePath$sep${e.name}',
                                );
                              }
                            },
                            onCreateFolder: () => _createNewFolderDialog(true),
                            onDeleteItem: () => _deleteDialog(true),
                          ),
                        ],
                      ),
              ),
            ],
          ),

          // Modal Transfer Progress Overlay
          if (_isTransferring)
            Container(
              color: Colors.black.withValues(alpha: 0.6),
              alignment: Alignment.center,
              child: Container(
                margin: const EdgeInsets.all(24),
                padding: const EdgeInsets.all(24.0),
                decoration: NeobrutalTheme.panel(color: NeobrutalTheme.paper),
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    const SizedBox(
                      width: 32,
                      height: 32,
                      child: CircularProgressIndicator(
                        strokeWidth: 3,
                        color: NeobrutalTheme.ink,
                      ),
                    ),
                    const SizedBox(height: 16),
                    Text(
                      _transferTitle,
                      style: const TextStyle(
                        fontWeight: FontWeight.w900,
                        fontSize: 16,
                        color: NeobrutalTheme.ink,
                      ),
                    ),
                    const SizedBox(height: 12),
                    SizedBox(
                      width: 250,
                      child: Container(
                        decoration: BoxDecoration(
                          border: Border.all(
                            color: NeobrutalTheme.ink,
                            width: NeobrutalTheme.compactBorderWidth,
                          ),
                          borderRadius: BorderRadius.circular(4),
                        ),
                        child: ClipRRect(
                          borderRadius: BorderRadius.circular(2),
                          child: LinearProgressIndicator(
                            value: _transferProgress,
                            minHeight: 12,
                            backgroundColor: NeobrutalTheme.surface,
                            valueColor: const AlwaysStoppedAnimation<Color>(
                              NeobrutalTheme.cyan,
                            ),
                          ),
                        ),
                      ),
                    ),
                    const SizedBox(height: 10),
                    Text(
                      _transferDetail,
                      style: const TextStyle(
                        fontSize: 12,
                        color: NeobrutalTheme.muted,
                        fontWeight: FontWeight.w700,
                      ),
                    ),
                  ],
                ),
              ),
            ),
        ],
      ),
    );
  }

  Widget _buildPane({
    required String title,
    required IconData icon,
    required String currentPath,
    required List<RemoteFileEntry> entries,
    required RemoteFileEntry? selectedEntry,
    required bool isLoading,
    required VoidCallback onNavigateUp,
    required ValueChanged<RemoteFileEntry> onEntrySelected,
    required ValueChanged<RemoteFileEntry> onEntryDoubleTap,
    required VoidCallback onCreateFolder,
    required VoidCallback onDeleteItem,
  }) {
    return Column(
      children: [
        // Path navigation bar
        Container(
          padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 6),
          decoration: const BoxDecoration(
            color: NeobrutalTheme.surface,
            border: Border(
              bottom: BorderSide(
                color: NeobrutalTheme.ink,
                width: NeobrutalTheme.borderWidth,
              ),
            ),
          ),
          child: Row(
            children: [
              IconButton(
                icon: const Icon(Icons.arrow_upward, size: 20),
                tooltip: 'Go Up',
                onPressed: onNavigateUp,
              ),
              Expanded(
                child: Text(
                  currentPath.isNotEmpty ? currentPath : 'Roots',
                  style: const TextStyle(
                    fontFamily: 'monospace',
                    fontSize: 13,
                    fontWeight: FontWeight.w700,
                  ),
                  overflow: TextOverflow.ellipsis,
                ),
              ),
              IconButton(
                icon: const Icon(Icons.create_new_folder, size: 20),
                tooltip: 'New Folder',
                onPressed: onCreateFolder,
              ),
              IconButton(
                icon: const Icon(Icons.delete_outline, size: 20),
                tooltip: 'Delete Selected',
                onPressed: selectedEntry != null ? onDeleteItem : null,
              ),
            ],
          ),
        ),

        // Files list
        Expanded(
          child: isLoading
              ? const Center(child: CircularProgressIndicator())
              : entries.isEmpty
              ? const Center(
                  child: Text(
                    'Empty directory',
                    style: TextStyle(
                      color: NeobrutalTheme.muted,
                      fontWeight: FontWeight.w700,
                    ),
                  ),
                )
              : ListView.builder(
                  itemCount: entries.length,
                  itemBuilder: (context, index) {
                    final entry = entries[index];
                    final isSelected = selectedEntry?.name == entry.name;

                    return Container(
                      margin: const EdgeInsets.symmetric(
                        horizontal: 8,
                        vertical: 4,
                      ),
                      decoration: NeobrutalTheme.compactPanel(
                        color: isSelected
                            ? NeobrutalTheme.cyan
                            : NeobrutalTheme.surface,
                        shadow: isSelected,
                      ),
                      child: Material(
                        color: Colors.transparent,
                        child: ListTile(
                          dense: true,
                          leading: Icon(
                            entry.isDir
                                ? Icons.folder
                                : _getFileIcon(entry.name),
                            color: NeobrutalTheme.ink,
                          ),
                          title: Text(
                            entry.name,
                            style: const TextStyle(
                              fontSize: 14,
                              fontWeight: FontWeight.w800,
                              color: NeobrutalTheme.ink,
                            ),
                          ),
                          trailing: Text(
                            entry.formattedSize,
                            style: const TextStyle(
                              fontSize: 12,
                              color: NeobrutalTheme.muted,
                              fontWeight: FontWeight.w700,
                            ),
                          ),
                          onTap: () => onEntrySelected(entry),
                          onLongPress: () => onEntryDoubleTap(entry),
                        ),
                      ),
                    );
                  },
                ),
        ),
      ],
    );
  }

  IconData _getFileIcon(String filename) {
    final ext = filename.split('.').last.toLowerCase();
    switch (ext) {
      case 'png':
      case 'jpg':
      case 'jpeg':
      case 'gif':
      case 'webp':
        return Icons.image;
      case 'mp4':
      case 'mkv':
      case 'avi':
      case 'mov':
        return Icons.movie;
      case 'mp3':
      case 'opus':
      case 'wav':
      case 'm4a':
        return Icons.music_note;
      case 'zip':
      case 'rar':
      case '7z':
      case 'tar':
      case 'gz':
        return Icons.archive;
      case 'pdf':
      case 'doc':
      case 'docx':
      case 'txt':
        return Icons.description;
      default:
        return Icons.insert_drive_file;
    }
  }
}
