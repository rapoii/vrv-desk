import 'dart:convert';
import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import '../services/audio_stream_player.dart';
import '../services/video_stream_player.dart';
import '../widgets/pin_dialog.dart';
import '../widgets/shortcut_bar.dart';

typedef WebSocketConnector = Future<WebSocket> Function(String url);

class MirrorView extends StatefulWidget {
  final String? hostIp;
  final int port;
  final String? initialPin;
  final String? signalingUrl;
  final String? targetDeviceId;
  final WebSocketConnector? webSocketConnector;
  final AudioStreamPlayer? audioPlayer;

  const MirrorView({
    super.key,
    this.hostIp,
    this.port = 53211,
    this.initialPin,
    this.signalingUrl,
    this.targetDeviceId,
    this.webSocketConnector,
    this.audioPlayer,
  }) : assert(hostIp != null || (signalingUrl != null && targetDeviceId != null),
            'Must provide either hostIp or both signalingUrl and targetDeviceId');

  @override
  State<MirrorView> createState() => _MirrorViewState();
}

class _MirrorViewState extends State<MirrorView> {
  WebSocket? _socket;
  Uint8List? _currentFrame;
  bool _isConnected = false;
  bool _isAuthenticated = false;
  bool _isAuthenticating = false;
  String? _authError;
  int _remainingAttempts = 3;
  bool _hasSentInitialPin = false;
  String _statusMessage = 'Connecting...';
  int _frameCount = 0;
  String _videoCodecName = 'Detecting...';
  BuildContext? _dialogContext;

  final TextEditingController _textController = TextEditingController();
  final FocusNode _keyboardFocusNode = FocusNode();
  bool _showShortcuts = false;

  late final AudioStreamPlayer _audioPlayer;
  late final VideoStreamPlayer _videoPlayer;
  int? _textureId;
  bool _isVideoDecoderInitialized = false;
  bool _isAudioMuted = false;

  @override
  void initState() {
    super.initState();
    _audioPlayer = widget.audioPlayer ?? AudioStreamPlayer();
    _videoPlayer = VideoStreamPlayer();
    _connect();
  }

  Future<void> _handleVh24Packet(List<int> data) async {
    final packet = VideoStreamPlayer.parseVh24Packet(data);
    if (packet == null) return;

    if (!_isVideoDecoderInitialized) {
      _isVideoDecoderInitialized = true;
      final id = await _videoPlayer.init(width: 1280, height: 720);
      if (mounted) {
        setState(() {
          _textureId = id;
          _videoCodecName = id != null ? 'H.264 (HW)' : 'JPEG';
        });
      }
    }

    if (_textureId != null) {
      await _videoPlayer.write(packet.nalPayload);
      if (mounted) {
        setState(() {
          _isConnected = true;
          _frameCount++;
          _videoCodecName = 'H.264 (HW)';
        });
      }
    }
  }

  Future<void> _connect() async {
    final isRemote = widget.targetDeviceId != null;
    final targetLabel = isRemote ? 'Device ID ${widget.targetDeviceId}' : '${widget.hostIp}:${widget.port}';

    setState(() {
      _isConnected = false;
      _isAuthenticated = false;
      _isAuthenticating = false;
      _authError = null;
      _remainingAttempts = 3;
      _hasSentInitialPin = false;
      _statusMessage = 'Connecting to $targetLabel...';
    });

    try {
      final connector = widget.webSocketConnector ?? WebSocket.connect;
      final connectUrl = isRemote
          ? widget.signalingUrl!
          : 'ws://${widget.hostIp}:${widget.port}';

      final ws = await connector(
        connectUrl,
      ).timeout(const Duration(seconds: 5));

      _socket = ws;

      if (isRemote) {
        setState(() {
          _statusMessage = 'Connecting via signaling to Device ${widget.targetDeviceId}...';
        });
        // Send connect_request with target_id
        final connectReq = jsonEncode({
          'type': 'connect_request',
          'target_id': widget.targetDeviceId,
          'client_name': 'Flutter-Client',
        });
        ws.add(connectReq);
      } else {
        setState(() {
          _isConnected = true;
          _statusMessage = 'Connected, waiting for host handshake...';
        });
      }

      ws.listen(
        (data) {
          if (data is List<int>) {
            if (AudioStreamPlayer.isVaudPacket(data)) {
              if (_isAuthenticated) {
                _audioPlayer.handleVaudPacket(data);
              }
            } else if (VideoStreamPlayer.isVh24Packet(data)) {
              if (_isAuthenticated) {
                _handleVh24Packet(data);
              }
            } else {
              setState(() {
                _isConnected = true;
                _currentFrame = Uint8List.fromList(data);
                _frameCount++;
                _videoCodecName = 'JPEG';
              });
            }
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
      if (!_isAuthenticated && event['type'] != 'auth_verify') {
        return;
      }
      _socket!.add(jsonEncode(event));
    }
  }

  void _sendPin(String pin) {
    setState(() {
      _isAuthenticating = true;
      _authError = null;
    });
    _sendInput({
      'type': 'auth_verify',
      'pin': pin,
    });
  }

  void _showPinDialog() {
    if (!mounted || _dialogContext != null) return;
    final label = widget.targetDeviceId != null
        ? 'Device ${widget.targetDeviceId}'
        : 'Host PC (${widget.hostIp})';
    showDialog<void>(
      context: context,
      barrierDismissible: true,
      builder: (dContext) {
        _dialogContext = dContext;
        return PinDialog(
          deviceName: label,
          onSubmitted: (enteredPin) {
            _sendPin(enteredPin);
          },
        );
      },
    ).then((_) {
      _dialogContext = null;
    });
  }

  void _dismissPinDialog() {
    if (_dialogContext != null && mounted) {
      final ctx = _dialogContext;
      _dialogContext = null;
      Navigator.of(ctx!).pop();
    }
  }

  void _handleAuthRequired() {
    if (widget.initialPin != null && widget.initialPin!.isNotEmpty && !_hasSentInitialPin) {
      _hasSentInitialPin = true;
      _sendPin(widget.initialPin!);
    } else {
      _showPinDialog();
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
        final type = json['type'];
        if (type == 'connect_error') {
          final reason = json['reason'] as String? ?? 'Signaling connection failed';
          setState(() {
            _isConnected = false;
            _statusMessage = 'Signaling error: $reason';
          });
          if (mounted) {
            ScaffoldMessenger.of(context).showSnackBar(
              SnackBar(
                content: Text('Remote Connect Error: $reason'),
                backgroundColor: Colors.red,
                duration: const Duration(seconds: 4),
                behavior: SnackBarBehavior.floating,
              ),
            );
          }
        } else if (type == 'auth_required') {
          setState(() {
            _isConnected = true;
          });
          _handleAuthRequired();
        } else if (type == 'auth_ok') {
          _dismissPinDialog();
          setState(() {
            _isAuthenticated = true;
            _isAuthenticating = false;
            _authError = null;
            _statusMessage = 'Connected & Authenticated';
          });
          if (mounted) {
            ScaffoldMessenger.of(context).hideCurrentSnackBar();
            ScaffoldMessenger.of(context).showSnackBar(
              const SnackBar(
                content: Text('Connected & Authenticated with Host PC'),
                duration: Duration(seconds: 2),
                behavior: SnackBarBehavior.floating,
              ),
            );
          }
        } else if (type == 'auth_failed') {
          final reason = json['reason'] as String? ?? 'Invalid PIN';
          final remaining = json['remaining_attempts'] as int? ?? 0;
          setState(() {
            _authError = reason;
            _remainingAttempts = remaining;
            _isAuthenticating = false;
          });
          if (mounted) {
            ScaffoldMessenger.of(context).hideCurrentSnackBar();
            if (remaining <= 0) {
              _dismissPinDialog();
              ScaffoldMessenger.of(context).showSnackBar(
                SnackBar(
                  content: Text('Authentication failed: $reason. Connection locked.'),
                  backgroundColor: Colors.red,
                  duration: const Duration(seconds: 3),
                  behavior: SnackBarBehavior.floating,
                ),
              );
              Future.delayed(const Duration(seconds: 2), () {
                if (mounted) {
                  Navigator.of(context).pop();
                }
              });
            } else {
              ScaffoldMessenger.of(context).showSnackBar(
                SnackBar(
                  content: Text('Authentication failed: $reason ($remaining attempts remaining)'),
                  backgroundColor: Colors.redAccent,
                  duration: const Duration(seconds: 3),
                  behavior: SnackBarBehavior.floating,
                ),
              );
              _showPinDialog();
            }
          }
        } else if (type == 'clipboard_sync' && json['text'] is String) {
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

  void _toggleAudioMute() {
    setState(() {
      _isAudioMuted = !_isAudioMuted;
    });
    _audioPlayer.setMuted(_isAudioMuted);
    if (mounted) {
      ScaffoldMessenger.of(context).hideCurrentSnackBar();
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text(_isAudioMuted ? '🔇 Audio muted' : '🔊 Audio unmuted'),
          duration: const Duration(seconds: 1),
          behavior: SnackBarBehavior.floating,
        ),
      );
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
    _audioPlayer.stop();
    _videoPlayer.dispose();
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
              child: _isAuthenticated && (_textureId != null || _currentFrame != null)
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
                            child: _textureId != null
                                ? Texture(textureId: _textureId!)
                                : Image.memory(
                                    _currentFrame!,
                                    gaplessPlayback: true,
                                    fit: BoxFit.contain,
                                  ),
                          ),
                        );
                      },
                    )
                  : Center(
                      child: Padding(
                        padding: const EdgeInsets.symmetric(horizontal: 24.0),
                        child: Column(
                          mainAxisSize: MainAxisSize.min,
                          children: [
                            if (_isConnected && !_isAuthenticated) ...[
                              Icon(
                                _authError != null ? Icons.lock_clock : Icons.lock_outline,
                                size: 54,
                                color: _authError != null ? Colors.orangeAccent : Colors.white70,
                              ),
                              const SizedBox(height: 16),
                              Text(
                                _isAuthenticating
                                    ? 'Verifying PIN with Host...'
                                    : (_authError != null
                                        ? 'Authentication Required ($_remainingAttempts attempts left)'
                                        : 'Authentication Required'),
                                style: const TextStyle(
                                  color: Colors.white,
                                  fontSize: 16,
                                  fontWeight: FontWeight.bold,
                                ),
                                textAlign: TextAlign.center,
                              ),
                              if (_authError != null) ...[
                                const SizedBox(height: 8),
                                Text(
                                  _authError!,
                                  style: const TextStyle(
                                    color: Colors.redAccent,
                                    fontSize: 13,
                                  ),
                                  textAlign: TextAlign.center,
                                ),
                              ],
                              const SizedBox(height: 20),
                              if (_remainingAttempts > 0)
                                ElevatedButton.icon(
                                  key: const Key('enter_pin_overlay_button'),
                                  onPressed: _showPinDialog,
                                  icon: const Icon(Icons.pin),
                                  label: const Text('Enter Host PIN'),
                                )
                              else
                                const Text(
                                  'Connection locked due to failed attempts.',
                                  style: TextStyle(color: Colors.red, fontSize: 13),
                                ),
                            ] else if (_isConnected && _isAuthenticated) ...[
                              const CircularProgressIndicator(color: Colors.white),
                              const SizedBox(height: 16),
                              const Text(
                                'Authenticated! Waiting for video stream...',
                                style: TextStyle(color: Colors.white70, fontSize: 14),
                                textAlign: TextAlign.center,
                              ),
                            ] else ...[
                              const Icon(Icons.cloud_off, size: 48, color: Colors.white54),
                              const SizedBox(height: 16),
                              Text(
                                _statusMessage,
                                style: const TextStyle(color: Colors.white70, fontSize: 14),
                                textAlign: TextAlign.center,
                              ),
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
                            color: !_isConnected
                                ? Colors.redAccent
                                : (!_isAuthenticated ? Colors.amberAccent : Colors.greenAccent),
                          ),
                        ),
                        const SizedBox(width: 8),
                        Text(
                          !_isConnected
                              ? 'OFFLINE'
                              : (!_isAuthenticated ? 'AUTH REQUIRED' : 'LIVE (${_frameCount}f)'),
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
                            icon: Icon(
                              _isAudioMuted ? Icons.volume_off : Icons.volume_up,
                              color: _isAudioMuted ? Colors.redAccent : Colors.white,
                            ),
                            tooltip: _isAudioMuted ? 'Unmute Host Audio' : 'Mute Host Audio',
                            onPressed: _toggleAudioMute,
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
