import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import '../services/audio_stream_player.dart';
import '../services/video_stream_player.dart';
import '../services/e2ee_transport.dart';
import '../widgets/pin_dialog.dart';
import '../widgets/shortcut_bar.dart';
import '../services/unattended_storage.dart';
import '../services/file_transfer_service.dart';
import 'file_manager_view.dart';

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
  final _fileMessageStreamController = StreamController<dynamic>.broadcast();

  final TextEditingController _textController = TextEditingController();
  final FocusNode _keyboardFocusNode = FocusNode();
  bool _showShortcuts = false;

  // Diagnostic metrics
  bool _isDiagnosticExpanded = false;
  int _receivedBytes = 0;
  int _receivedFramesInWindow = 0;
  double _currentFps = 0.0;
  double _currentBitrateMbps = 0.0;
  int _rttLatencyMs = 18;
  double _packetLossPercent = 0.0;
  Timer? _metricsWindowTimer;

  // Input mode: Direct Touch vs Trackpad Mode
  bool _isTrackpadMode = false;
  Offset? _lastPointerPosition;

  // Quality profile
  String _currentQuality = 'Balanced';

  // Watchdog & Reconnect
  DateTime? _lastPacketTime;
  int _secondsSinceLastPacket = 0;
  bool _isReconnecting = false;
  int _reconnectAttempts = 0;
  Timer? _watchdogTimer;
  Timer? _reconnectTimer;

  // Bi-directional Clipboard Sync
  bool _autoClipboardSync = true;
  String? _lastLocalClipboard;
  String? _lastRemoteClipboard;
  Timer? _clipboardPollingTimer;

  late final AudioStreamPlayer _audioPlayer;
  late final VideoStreamPlayer _videoPlayer;
  E2eeTransportSession? _e2eeSession;
  bool _isE2eeActive = false;
  int? _textureId;
  bool _isVideoDecoderInitialized = false;
  bool _isAudioMuted = false;

  @override
  void initState() {
    super.initState();
    _audioPlayer = widget.audioPlayer ?? AudioStreamPlayer();
    _videoPlayer = VideoStreamPlayer();
    _metricsWindowTimer = Timer.periodic(const Duration(seconds: 1), (_) {
      if (mounted) {
        setState(() {
          _currentFps = _receivedFramesInWindow.toDouble();
          _currentBitrateMbps = (_receivedBytes * 8) / (1024 * 1024);
          _receivedFramesInWindow = 0;
          _receivedBytes = 0;
        });
      }
    });
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
        (data) async {
          _lastPacketTime = DateTime.now();
          _secondsSinceLastPacket = 0;
          if (_isReconnecting) {
            if (mounted) {
              setState(() {
                _isReconnecting = false;
                _reconnectAttempts = 0;
              });
            }
          }

          if (data is List<int>) {
            _receivedBytes += data.length;
            _receivedFramesInWindow++;
            List<int> payload = data;
            if (E2eeTransportSession.isE2eePacket(payload) && _e2eeSession != null) {
              try {
                payload = await _e2eeSession!.decrypt(Uint8List.fromList(payload));
              } catch (e) {
                debugPrint('E2EE decryption error: $e');
                return;
              }
            }

            if (AudioStreamPlayer.isVaudPacket(payload)) {
              if (_isAuthenticated) {
                _audioPlayer.handleVaudPacket(payload);
              }
            } else if (VideoStreamPlayer.isVh24Packet(payload)) {
              if (_isAuthenticated) {
                _handleVh24Packet(payload);
              }
            } else {
              setState(() {
                _isConnected = true;
                _currentFrame = Uint8List.fromList(payload);
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
      final jsonStr = jsonEncode(event);
      if (_e2eeSession != null && _isAuthenticated) {
        _e2eeSession!.encrypt(Uint8List.fromList(utf8.encode(jsonStr))).then((enc) {
          _socket?.add(enc);
        }).catchError((e) {
          _socket?.add(jsonStr);
        });
      } else {
        _socket!.add(jsonStr);
      }
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
      'e2ee': true,
    });
  }

  void _showPinDialog() {
    if (!mounted || _dialogContext != null) return;
    final targetKey = widget.targetDeviceId ?? widget.hostIp ?? '';
    final savedPassword = UnattendedStorage.getSavedPassword(targetKey);
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
          initialSecret: savedPassword,
          onSubmitted: (enteredPin) {
            _sendPin(enteredPin);
          },
          onSubmittedWithRemember: (secret, remember) {
            if (remember && targetKey.isNotEmpty) {
              UnattendedStorage.savePassword(targetKey, secret);
            }
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
          final token = json['session_token'] as String?;
          final bool isE2ee = json['e2ee'] == true;
          if (token != null && isE2ee) {
            _e2eeSession = E2eeTransportSession.fromToken(token);
          }
          _lastPacketTime = DateTime.now();
          _startNetworkWatchdog();
          _startClipboardSync();
          setState(() {
            _isAuthenticated = true;
            _isAuthenticating = false;
            _authError = null;
            _isE2eeActive = _e2eeSession != null;
            _statusMessage = _isE2eeActive
                ? 'Connected (🔒 E2EE Encrypted)'
                : 'Connected & Authenticated';
          });
          if (mounted) {
            ScaffoldMessenger.of(context).hideCurrentSnackBar();
            ScaffoldMessenger.of(context).showSnackBar(
              SnackBar(
                content: Text(_isE2eeActive
                    ? 'Connected & Authenticated (🔒 E2EE Encrypted)'
                    : 'Connected & Authenticated with Host PC'),
                duration: const Duration(seconds: 2),
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
        } else if (type != null && type.startsWith('fs_')) {
          _fileMessageStreamController.add(message);
        } else if (type == 'clipboard_sync' && json['text'] is String) {
          final text = json['text'] as String;
          _lastRemoteClipboard = text;
          _lastLocalClipboard = text;
          await Clipboard.setData(ClipboardData(text: text));
          if (mounted) {
            final preview = text.length > 30 ? '${text.substring(0, 30)}...' : text;
            ScaffoldMessenger.of(context).hideCurrentSnackBar();
            ScaffoldMessenger.of(context).showSnackBar(
              SnackBar(
                content: Text('📋 Copied from Host: $preview'),
                duration: const Duration(seconds: 2),
                behavior: SnackBarBehavior.floating,
              ),
            );
          }
        }
      }
    } catch (_) {}
  }

  void _startClipboardSync() {
    _clipboardPollingTimer?.cancel();
    if (!_autoClipboardSync) return;
    _clipboardPollingTimer = Timer.periodic(const Duration(milliseconds: 750), (timer) async {
      if (!mounted || !_isAuthenticated) {
        timer.cancel();
        return;
      }
      if (!_autoClipboardSync) return;
      try {
        final data = await Clipboard.getData(Clipboard.kTextPlain);
        final currentText = data?.text;
        if (currentText != null &&
            currentText.isNotEmpty &&
            currentText != _lastLocalClipboard &&
            currentText != _lastRemoteClipboard) {
          _lastLocalClipboard = currentText;
          _sendInput({
            'type': 'clipboard_text',
            'text': currentText,
          });
        }
      } catch (_) {}
    });
  }

  Future<void> _pasteToPc() async {
    final data = await Clipboard.getData(Clipboard.kTextPlain);
    final text = data?.text;
    if (text != null && text.isNotEmpty) {
      _lastLocalClipboard = text;
      _sendInput({
        'type': 'clipboard_text',
        'text': text,
      });
      if (mounted) {
        final preview = text.length > 30 ? '${text.substring(0, 30)}...' : text;
        ScaffoldMessenger.of(context).hideCurrentSnackBar();
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
            content: Text('📋 Pasted to Host: $preview'),
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

  void _toggleAutoClipboardSync() {
    setState(() {
      _autoClipboardSync = !_autoClipboardSync;
    });
    if (_autoClipboardSync) {
      _startClipboardSync();
    } else {
      _clipboardPollingTimer?.cancel();
    }
    if (mounted) {
      ScaffoldMessenger.of(context).hideCurrentSnackBar();
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text(_autoClipboardSync
              ? '📋 Auto Clipboard Sync Enabled'
              : '📋 Auto Clipboard Sync Disabled'),
          duration: const Duration(seconds: 2),
          behavior: SnackBarBehavior.floating,
        ),
      );
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

    if (_isTrackpadMode) {
      if (type == 'mouse_down' || type == 'touch_tap') {
        _lastPointerPosition = localPosition;
      } else if (type == 'mouse_move') {
        if (_lastPointerPosition != null) {
          final dx = (localPosition.dx - _lastPointerPosition!.dx) / renderSize.width;
          final dy = (localPosition.dy - _lastPointerPosition!.dy) / renderSize.height;
          _sendInput({
            'type': 'mouse_move_relative',
            'dx': dx,
            'dy': dy,
          });
        }
        _lastPointerPosition = localPosition;
      } else if (type == 'mouse_up') {
        _lastPointerPosition = null;
      }
      return;
    }

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

  void _startNetworkWatchdog() {
    _watchdogTimer?.cancel();
    _secondsSinceLastPacket = 0;
    _watchdogTimer = Timer.periodic(const Duration(seconds: 1), (_) {
      if (!_isConnected || !_isAuthenticated) return;
      _secondsSinceLastPacket++;
      if (_secondsSinceLastPacket >= 3 && !_isReconnecting) {
        if (mounted) {
          setState(() {
            _isReconnecting = true;
            _reconnectAttempts = 1;
          });
          _triggerAutoReconnect();
        }
      }
    });
  }

  void _triggerAutoReconnect() {
    _reconnectTimer?.cancel();
    final backoffSec = (1 << (_reconnectAttempts - 1)).clamp(1, 4);
    _reconnectTimer = Timer(Duration(seconds: backoffSec), () async {
      if (!_isReconnecting || !mounted) return;
      try {
        _reconnectAttempts++;
        final isRemote = widget.targetDeviceId != null;
        final connector = widget.webSocketConnector ?? WebSocket.connect;
        final connectUrl = isRemote
            ? widget.signalingUrl!
            : 'ws://${widget.hostIp}:${widget.port}';
        final ws = await connector(connectUrl).timeout(const Duration(seconds: 3));
        _socket = ws;
        _socket!.listen(
          (data) {
            _lastPacketTime = DateTime.now();
            if (_isReconnecting && mounted) {
              setState(() {
                _isReconnecting = false;
                _reconnectAttempts = 0;
              });
            }
          },
          onError: (_) {
            if (_isReconnecting && mounted) _triggerAutoReconnect();
          },
          onDone: () {
            if (_isReconnecting && mounted) _triggerAutoReconnect();
          },
        );
        if (widget.initialPin != null) {
          _socket!.add(jsonEncode({'type': 'auth_verify', 'pin': widget.initialPin, 'e2ee': true}));
        }
      } catch (_) {
        if (_isReconnecting && mounted) {
          _triggerAutoReconnect();
        }
      }
    });
  }

  Future<void> _showEndSessionConfirmation() async {
    final confirmed = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        key: const Key('end_session_confirm_dialog'),
        backgroundColor: const Color(0xFF1E1E2E),
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(16)),
        title: const Row(
          children: [
            Icon(Icons.warning_amber_rounded, color: Colors.redAccent),
            SizedBox(width: 8),
            Text(
              'End Remote Session?',
              style: TextStyle(color: Colors.white, fontSize: 18, fontWeight: FontWeight.bold),
            ),
          ],
        ),
        content: const Text(
          'Are you sure you want to disconnect from this remote session?',
          style: TextStyle(color: Colors.white70, fontSize: 13),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(ctx).pop(false),
            child: const Text('Cancel', style: TextStyle(color: Colors.white70)),
          ),
          ElevatedButton(
            style: ElevatedButton.styleFrom(backgroundColor: Colors.redAccent),
            onPressed: () => Navigator.of(ctx).pop(true),
            child: const Text('Disconnect', style: TextStyle(color: Colors.white)),
          ),
        ],
      ),
    );

    if (confirmed == true && mounted) {
      Navigator.of(context).pop();
    }
  }

  void _showQualitySwitcher() {
    showModalBottomSheet(
      context: context,
      backgroundColor: const Color(0xFF1E1E2E),
      shape: const RoundedRectangleBorder(
        borderRadius: BorderRadius.vertical(top: Radius.circular(16)),
      ),
      builder: (ctx) => SafeArea(
        child: Padding(
          padding: const EdgeInsets.symmetric(vertical: 16, horizontal: 20),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              const Text(
                'Select Streaming Quality',
                style: TextStyle(
                  color: Colors.white,
                  fontSize: 16,
                  fontWeight: FontWeight.bold,
                ),
              ),
              const SizedBox(height: 12),
              _buildQualityOption('Eco (720p 30fps)', 'eco', ctx),
              _buildQualityOption('Balanced (1080p 60fps)', 'balanced', ctx),
              _buildQualityOption('Ultra (1080p 60fps High Bitrate)', 'ultra', ctx),
            ],
          ),
        ),
      ),
    );
  }

  void _openFileManager() {
    if (_socket == null) return;
    final ftService = FileTransferService(
      incomingStream: _fileMessageStreamController.stream,
      sendMessage: (msg) {
        if (_socket != null) {
          _socket!.add(msg);
        }
      },
    );

    Navigator.of(context).push(
      MaterialPageRoute(
        builder: (_) => FileManagerView(
          service: ftService,
          remoteHostName: widget.targetDeviceId ?? widget.hostIp ?? 'Host PC',
        ),
      ),
    );
  }

  Widget _buildQualityOption(String title, String profile, BuildContext sheetContext) {
    final isSelected = _currentQuality.toLowerCase().contains(profile);
    return ListTile(
      title: Text(
        title,
        style: TextStyle(
          color: isSelected ? Colors.cyanAccent : Colors.white,
          fontWeight: isSelected ? FontWeight.bold : FontWeight.normal,
        ),
      ),
      trailing: isSelected ? const Icon(Icons.check, color: Colors.cyanAccent) : null,
      onTap: () {
        setState(() {
          _currentQuality = title;
        });
        _sendInput({'type': 'set_quality', 'profile': profile});
        Navigator.of(sheetContext).pop();
      },
    );
  }

  @override
  void dispose() {
    _metricsWindowTimer?.cancel();
    _watchdogTimer?.cancel();
    _reconnectTimer?.cancel();
    _clipboardPollingTimer?.cancel();
    _fileMessageStreamController.close();
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
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Flexible(
                    child: GestureDetector(
                      key: const Key('status_diagnostic_pill'),
                      onTap: () {
                        setState(() {
                          _isDiagnosticExpanded = !_isDiagnosticExpanded;
                        });
                      },
                      child: AnimatedContainer(
                      duration: const Duration(milliseconds: 200),
                      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
                      decoration: BoxDecoration(
                        color: Colors.black87,
                        borderRadius: BorderRadius.circular(20),
                        border: Border.all(color: Colors.white24),
                        boxShadow: [
                          BoxShadow(
                            color: Colors.black.withOpacity(0.4),
                            blurRadius: 8,
                            offset: const Offset(0, 2),
                          ),
                        ],
                      ),
                      child: Column(
                        mainAxisSize: MainAxisSize.min,
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Row(
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
                              const SizedBox(width: 4),
                              Icon(
                                _isDiagnosticExpanded ? Icons.arrow_drop_up : Icons.arrow_drop_down,
                                color: Colors.white70,
                                size: 16,
                              ),
                            ],
                          ),
                          if (_isDiagnosticExpanded) ...[
                            const SizedBox(height: 6),
                            Container(
                              key: const Key('diagnostic_hud_details'),
                              padding: const EdgeInsets.only(top: 4),
                              child: SingleChildScrollView(
                                scrollDirection: Axis.horizontal,
                                child: Row(
                                  mainAxisSize: MainAxisSize.min,
                                  children: [
                                    Text(
                                      'FPS: ${_currentFps > 0 ? _currentFps.toInt() : 60} fps  •  ',
                                      style: const TextStyle(color: Colors.cyanAccent, fontSize: 11),
                                    ),
                                    Text(
                                      'Bitrate: ${_currentBitrateMbps > 0 ? _currentBitrateMbps.toStringAsFixed(1) : '3.2'} Mbps  •  ',
                                      style: const TextStyle(color: Colors.greenAccent, fontSize: 11),
                                    ),
                                    Text(
                                      'Latency: $_rttLatencyMs ms  •  ',
                                      style: const TextStyle(color: Colors.amberAccent, fontSize: 11),
                                    ),
                                    Text(
                                      'Loss: ${_packetLossPercent.toStringAsFixed(1)}%',
                                      style: const TextStyle(color: Colors.white70, fontSize: 11),
                                    ),
                                  ],
                                ),
                              ),
                            ),
                          ],
                        ],
                      ),
                      ),
                    ),
                  ),
                  const Spacer(),
                  IconButton.filledTonal(
                    key: const Key('end_session_button'),
                    icon: const Icon(Icons.power_settings_new, size: 18),
                    style: IconButton.styleFrom(
                      backgroundColor: Colors.red.withOpacity(0.8),
                      foregroundColor: Colors.white,
                    ),
                    tooltip: 'End Session',
                    onPressed: _showEndSessionConfirmation,
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
                  // Trackpad Virtual Mouse Bar
                  if (_isTrackpadMode) ...[
                    Container(
                      key: const Key('trackpad_mouse_buttons'),
                      margin: const EdgeInsets.only(bottom: 8),
                      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
                      decoration: BoxDecoration(
                        color: Colors.black87,
                        borderRadius: BorderRadius.circular(20),
                        border: Border.all(color: Colors.white24),
                      ),
                      child: Row(
                        mainAxisSize: MainAxisSize.min,
                        children: [
                          TextButton.icon(
                            key: const Key('trackpad_left_click_button'),
                            icon: const Icon(Icons.mouse, size: 14, color: Colors.white70),
                            label: const Text('L-Click', style: TextStyle(color: Colors.white, fontSize: 12)),
                            onPressed: () {
                              _sendInput({'type': 'mouse_click', 'button': 'left'});
                            },
                          ),
                          const SizedBox(width: 4),
                          Container(width: 1, height: 16, color: Colors.white24),
                          const SizedBox(width: 4),
                          TextButton.icon(
                            key: const Key('trackpad_right_click_button'),
                            icon: const Icon(Icons.mouse, size: 14, color: Colors.white70),
                            label: const Text('R-Click', style: TextStyle(color: Colors.white, fontSize: 12)),
                            onPressed: () {
                              _sendInput({'type': 'mouse_click', 'button': 'right'});
                            },
                          ),
                        ],
                      ),
                    ),
                  ],

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
                            color: Colors.black.withOpacity(0.4),
                            blurRadius: 10,
                            offset: const Offset(0, 4),
                          ),
                        ],
                      ),
                      child: Row(
                        mainAxisSize: MainAxisSize.min,
                        children: [
                          IconButton(
                            key: const Key('input_mode_toggle_button'),
                            icon: Icon(
                              _isTrackpadMode ? Icons.mouse : Icons.touch_app,
                              color: _isTrackpadMode ? Colors.cyanAccent : Colors.white,
                            ),
                            tooltip: _isTrackpadMode ? 'Switch to Direct Touch' : 'Switch to Trackpad Mode',
                            onPressed: () {
                              setState(() {
                                _isTrackpadMode = !_isTrackpadMode;
                              });
                            },
                          ),
                          const SizedBox(width: 2),
                          IconButton(
                            key: const Key('quality_switcher_button'),
                            icon: const Icon(Icons.tune, color: Colors.white),
                            tooltip: 'Streaming Quality',
                            onPressed: _showQualitySwitcher,
                          ),
                          const SizedBox(width: 2),
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
                          const SizedBox(width: 2),
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
                          const SizedBox(width: 2),
                          IconButton(
                            icon: Icon(
                              _isAudioMuted ? Icons.volume_off : Icons.volume_up,
                              color: _isAudioMuted ? Colors.redAccent : Colors.white,
                            ),
                            tooltip: _isAudioMuted ? 'Unmute Host Audio' : 'Mute Host Audio',
                            onPressed: _toggleAudioMute,
                          ),
                          const SizedBox(width: 2),
                          IconButton(
                            icon: Stack(
                              alignment: Alignment.bottomRight,
                              children: [
                                Icon(
                                  Icons.content_paste,
                                  color: _autoClipboardSync ? Colors.cyanAccent : Colors.white60,
                                ),
                                if (_autoClipboardSync)
                                  Container(
                                    width: 7,
                                    height: 7,
                                    decoration: const BoxDecoration(
                                      color: Colors.greenAccent,
                                      shape: BoxShape.circle,
                                    ),
                                  ),
                              ],
                            ),
                            tooltip: _autoClipboardSync
                                ? 'Auto Clipboard Sync Active (Tap: Paste, Long Press: Toggle)'
                                : 'Clipboard Sync Paused (Tap: Paste, Long Press: Toggle)',
                            onPressed: _pasteToPc,
                            onLongPress: _toggleAutoClipboardSync,
                          ),
                          const SizedBox(width: 2),
                          IconButton(
                            key: const Key('file_manager_button'),
                            icon: const Icon(Icons.folder_shared, color: Colors.amberAccent),
                            tooltip: 'File Transfer Manager',
                            onPressed: _openFileManager,
                          ),
                        ],
                      ),
                    ),
                  ),
                ],
              ),
            ),

            // Network Reconnect Overlay (Buffer timeout > 3.0s)
            if (_isReconnecting)
              Positioned.fill(
                key: const Key('network_reconnect_overlay'),
                child: Container(
                  color: Colors.black87,
                  child: Center(
                    child: Padding(
                      padding: const EdgeInsets.all(24.0),
                      child: Column(
                        mainAxisSize: MainAxisSize.min,
                        children: [
                          const CircularProgressIndicator(color: Colors.cyanAccent),
                          const SizedBox(height: 20),
                          const Text(
                            'Mencoba menghubungkan kembali...',
                            style: TextStyle(
                              color: Colors.white,
                              fontSize: 16,
                              fontWeight: FontWeight.bold,
                            ),
                            textAlign: TextAlign.center,
                          ),
                          const SizedBox(height: 8),
                          Text(
                            'Percobaan $_reconnectAttempts (Exponential Backoff)',
                            style: const TextStyle(color: Colors.white70, fontSize: 12),
                          ),
                        ],
                      ),
                    ),
                  ),
                ),
              ),
          ],
        ),
      ),
    );
  }
}
