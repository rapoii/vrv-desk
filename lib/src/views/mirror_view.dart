import 'dart:convert';
import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import '../widgets/shortcut_bar.dart';

class MirrorView extends StatefulWidget {
  final String hostIp;
  final int port;

  const MirrorView({
    super.key,
    required this.hostIp,
    this.port = 53211,
  });

  @override
  State<MirrorView> createState() => _MirrorViewState();
}

class _MirrorViewState extends State<MirrorView> {
  WebSocket? _socket;
  Uint8List? _currentFrame;
  bool _isConnected = false;
  String _statusMessage = 'Connecting...';
  int _frameCount = 0;

  final TextEditingController _textController = TextEditingController();
  final FocusNode _keyboardFocusNode = FocusNode();
  bool _showShortcuts = false;

  @override
  void initState() {
    super.initState();
    _connect();
  }

  Future<void> _connect() async {
    setState(() {
      _statusMessage = 'Connecting to ${widget.hostIp}:${widget.port}...';
    });

    try {
      final ws = await WebSocket.connect(
        'ws://${widget.hostIp}:${widget.port}',
      ).timeout(const Duration(seconds: 5));

      _socket = ws;
      setState(() {
        _isConnected = true;
        _statusMessage = 'Connected';
      });

      ws.listen(
        (data) {
          if (data is List<int>) {
            setState(() {
              _currentFrame = Uint8List.fromList(data);
              _frameCount++;
            });
          } else if (data is String) {
            _handleTextMessage(data);
          }
        },
        onError: (err) {
          if (mounted) {
            setState(() {
              _isConnected = false;
              _statusMessage = 'Connection error: $err';
            });
          }
        },
        onDone: () {
          if (mounted) {
            setState(() {
              _isConnected = false;
              _statusMessage = 'Disconnected';
            });
          }
        },
      );
    } catch (e) {
      if (mounted) {
        setState(() {
          _isConnected = false;
          _statusMessage = 'Failed to connect: $e';
        });
      }
    }
  }

  void _sendInput(Map<String, dynamic> event) {
    if (_socket != null && _isConnected) {
      _socket!.add(jsonEncode(event));
    }
  }

  void _sendText(String text) {
    if (text.isEmpty) return;
    _sendInput({
      'type': 'type_text',
      'text': text,
    });
  }

  void _sendShortcut(String shortcutName) {
    _sendInput({
      'type': 'shortcut',
      'name': shortcutName,
    });
  }

  Future<void> _handleTextMessage(String message) async {
    try {
      final json = jsonDecode(message);
      if (json is Map<String, dynamic>) {
        if (json['type'] == 'clipboard_sync' && json['text'] is String) {
          final text = json['text'] as String;
          await Clipboard.setData(ClipboardData(text: text));
          if (mounted) {
            final preview = text.length > 30 ? '${text.substring(0, 30)}...' : text;
            ScaffoldMessenger.of(context).hideCurrentSnackBar();
            ScaffoldMessenger.of(context).showSnackBar(
              SnackBar(
                content: Text('📋 Copied from PC: $preview'),
                duration: const Duration(seconds: 2),
                behavior: SnackBarBehavior.floating,
              ),
            );
          }
        }
      }
    } catch (_) {}
  }

  Future<void> _pasteToPc() async {
    final data = await Clipboard.getData(Clipboard.kTextPlain);
    final text = data?.text;
    if (text != null && text.isNotEmpty) {
      _sendInput({
        'type': 'clipboard_text',
        'text': text,
      });
      if (mounted) {
        final preview = text.length > 30 ? '${text.substring(0, 30)}...' : text;
        ScaffoldMessenger.of(context).hideCurrentSnackBar();
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
            content: Text('📋 Pasted to PC: $preview'),
            duration: const Duration(seconds: 2),
            behavior: SnackBarBehavior.floating,
          ),
        );
      }
    } else {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(
            content: Text('Clipboard is empty'),
            duration: Duration(seconds: 1),
            behavior: SnackBarBehavior.floating,
          ),
        );
      }
    }
  }

  void _toggleKeyboard() {
    if (_keyboardFocusNode.hasFocus) {
      _keyboardFocusNode.unfocus();
    } else {
      _keyboardFocusNode.requestFocus();
    }
  }

  void _toggleShortcuts() {
    setState(() {
      _showShortcuts = !_showShortcuts;
    });
  }

  void _handlePointerEvent(Offset localPosition, Size renderSize, String type, {String? button}) {
    if (renderSize.width <= 0 || renderSize.height <= 0) return;

    // Calculate exact letterboxed 16:9 image rect inside container
    const imageAspect = 16.0 / 9.0;
    final containerAspect = renderSize.width / renderSize.height;
    double renderW, renderH, offsetX, offsetY;

    if (containerAspect > imageAspect) {
      renderH = renderSize.height;
      renderW = renderH * imageAspect;
      offsetX = (renderSize.width - renderW) / 2;
      offsetY = 0;
    } else {
      renderW = renderSize.width;
      renderH = renderW / imageAspect;
      offsetX = 0;
      offsetY = (renderSize.height - renderH) / 2;
    }

    final normX = ((localPosition.dx - offsetX) / renderW).clamp(0.0, 1.0);
    final normY = ((localPosition.dy - offsetY) / renderH).clamp(0.0, 1.0);

    final payload = <String, dynamic>{
      'type': type,
      'x': normX,
      'y': normY,
    };
    if (button != null) {
      payload['button'] = button;
    }
    _sendInput(payload);
  }

  @override
  void dispose() {
    _socket?.close();
    _textController.dispose();
    _keyboardFocusNode.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      backgroundColor: Colors.black,
      body: SafeArea(
        child: Stack(
          children: [
            // Video Canvas & Interactive Touch Layer
            Positioned.fill(
              child: _currentFrame != null
                  ? LayoutBuilder(
                      builder: (context, constraints) {
                        final size = Size(constraints.maxWidth, constraints.maxHeight);
                        return Listener(
                          behavior: HitTestBehavior.opaque,
                          onPointerDown: (event) {
                            _handlePointerEvent(event.localPosition, size, 'touch_tap');
                            _handlePointerEvent(event.localPosition, size, 'mouse_down');
                          },
                          onPointerMove: (event) {
                            _handlePointerEvent(event.localPosition, size, 'mouse_move');
                          },
                          onPointerUp: (event) {
                            _handlePointerEvent(event.localPosition, size, 'mouse_up');
                          },
                          child: Center(
                            child: Image.memory(
                              _currentFrame!,
                              gaplessPlayback: true,
                              fit: BoxFit.contain,
                            ),
                          ),
                        );

                      },
                    )
                  : Center(
                      child: Column(
                        mainAxisSize: MainAxisSize.min,
                        children: [
                          if (_isConnected)
                            const CircularProgressIndicator(color: Colors.white)
                          else
                            const Icon(Icons.cloud_off, size: 48, color: Colors.white54),
                          const SizedBox(height: 16),
                          Text(
                            _statusMessage,
                            style: const TextStyle(color: Colors.white70, fontSize: 14),
                            textAlign: TextAlign.center,
                          ),
                          if (!_isConnected) ...[
                            const SizedBox(height: 20),
                            ElevatedButton.icon(
                              onPressed: _connect,
                              icon: const Icon(Icons.refresh),
                              label: const Text('Retry Connection'),
                            ),
                          ],
                        ],
                      ),
                    ),
            ),

            // Hidden TextField to hook into mobile soft keyboard
            Positioned(
              left: -9999,
              top: -9999,
              child: SizedBox(
                width: 1,
                height: 1,
                child: TextField(
                  controller: _textController,
                  focusNode: _keyboardFocusNode,
                  autocorrect: false,
                  enableSuggestions: false,
                  keyboardType: TextInputType.text,
                  onChanged: (val) {
                    if (val.isNotEmpty) {
                      _sendText(val);
                      _textController.clear();
                    }
                  },
                ),
              ),
            ),

            // Minimalist Floating Top Control Bar
            Positioned(
              top: 12,
              left: 12,
              right: 12,
              child: Row(
                children: [
                  Container(
                    padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
                    decoration: BoxDecoration(
                      color: Colors.black54,
                      borderRadius: BorderRadius.circular(20),
                      border: Border.all(color: Colors.white24),
                    ),
                    child: Row(
                      mainAxisSize: MainAxisSize.min,
                      children: [
                        Container(
                          width: 8,
                          height: 8,
                          decoration: BoxDecoration(
                            shape: BoxShape.circle,
                            color: _isConnected ? Colors.greenAccent : Colors.redAccent,
                          ),
                        ),
                        const SizedBox(width: 8),
                        Text(
                          _isConnected ? 'LIVE (${_frameCount}f)' : 'OFFLINE',
                          style: const TextStyle(
                            color: Colors.white,
                            fontSize: 12,
                            fontWeight: FontWeight.bold,
                          ),
                        ),
                      ],
                    ),
                  ),
                  const Spacer(),
                  IconButton.filledTonal(
                    icon: const Icon(Icons.close, size: 18),
                    style: IconButton.styleFrom(
                      backgroundColor: Colors.black54,
                      foregroundColor: Colors.white,
                    ),
                    onPressed: () => Navigator.of(context).pop(),
                  ),
                ],
              ),
            ),

            // Floating Bottom Control Dock & Shortcut Bar
            Positioned(
              bottom: 16,
              left: 16,
              right: 16,
              child: Column(
                mainAxisSize: MainAxisSize.min,
                children: [
                  if (_showShortcuts) ...[
                    ShortcutBar(
                      onShortcutPressed: _sendShortcut,
                    ),
                    const SizedBox(height: 10),
                  ],
                  Center(
                    child: Container(
                      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
                      decoration: BoxDecoration(
                        color: Colors.black87,
                        borderRadius: BorderRadius.circular(30),
                        border: Border.all(color: Colors.white24),
                        boxShadow: [
                          BoxShadow(
                            color: Colors.black.withValues(alpha: 0.4),
                            blurRadius: 10,
                            offset: const Offset(0, 4),
                          ),
                        ],
                      ),
                      child: Row(
                        mainAxisSize: MainAxisSize.min,
                        children: [
                          IconButton(
                            icon: Icon(
                              Icons.keyboard,
                              color: _keyboardFocusNode.hasFocus
                                  ? Theme.of(context).colorScheme.primary
                                  : Colors.white,
                            ),
                            tooltip: 'Toggle Soft Keyboard',
                            onPressed: _toggleKeyboard,
                          ),
                          const SizedBox(width: 4),
                          IconButton(
                            icon: Icon(
                              Icons.grid_view,
                              color: _showShortcuts
                                  ? Theme.of(context).colorScheme.primary
                                  : Colors.white,
                            ),
                            tooltip: 'Toggle Shortcuts Bar',
                            onPressed: _toggleShortcuts,
                          ),
                          const SizedBox(width: 4),
                          IconButton(
                            icon: const Icon(
                              Icons.content_paste,
                              color: Colors.white,
                            ),
                            tooltip: 'Paste Phone Clipboard to PC',
                            onPressed: _pasteToPc,
                          ),
                        ],
                      ),
                    ),
                  ),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}
