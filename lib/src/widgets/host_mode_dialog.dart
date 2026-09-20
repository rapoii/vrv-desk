import 'dart:async';

import 'package:flutter/material.dart';
import 'package:qr_flutter/qr_flutter.dart';

import '../services/android_host_service.dart';
import '../services/qr_pairing_service.dart';
import '../theme/neobrutalist_theme.dart';

/// Modal dialog for Android Host screen broadcasting, permission handling,
/// PIN display, live connection metrics, and QR sharing.
class HostModeDialog extends StatefulWidget {
  final AndroidHostService? hostService;
  final String deviceId;
  final String? deviceName;
  final int port;
  final bool enableMetricsTimer;

  const HostModeDialog({
    super.key,
    this.hostService,
    required this.deviceId,
    this.deviceName,
    this.port = AndroidHostService.defaultPort,
    this.enableMetricsTimer = true,
  });

  @override
  State<HostModeDialog> createState() => _HostModeDialogState();
}

class _HostModeDialogState extends State<HostModeDialog> {
  late final AndroidHostService _hostService;
  bool _isAccessibilityEnabled = true;
  bool _isAudioSupported = true;
  bool _isAdbAvailable = false;
  double _maxRefreshRate = 60.0;
  bool _useAdbMode = false;
  bool _isStartingOrStopping = false;
  Timer? _metricsTimer;

  @override
  void initState() {
    super.initState();
    _hostService = widget.hostService ?? AndroidHostService();
    _checkAccessibility();
    _checkAudioSupport();
    _checkAdbStatus();
    _startMetricsTimer();
  }

  @override
  void dispose() {
    _metricsTimer?.cancel();
    super.dispose();
  }

  void _startMetricsTimer() {
    if (!widget.enableMetricsTimer) return;
    _metricsTimer = Timer.periodic(const Duration(milliseconds: 1000), (_) {
      if (mounted && _hostService.isRunning) {
        setState(() {});
      }
    });
  }

  Future<void> _checkAccessibility() async {
    final enabled = await _hostService.isAccessibilityEnabled();
    if (mounted) {
      setState(() {
        _isAccessibilityEnabled = enabled;
      });
    }
  }

  Future<void> _checkAudioSupport() async {
    final supported = await _hostService.isInternalAudioSupported();
    if (mounted) {
      setState(() {
        _isAudioSupported = supported;
      });
    }
  }

  Future<void> _checkAdbStatus() async {
    final available = await _hostService.isAdbHighPerformanceAvailable();
    final refreshRate = await _hostService.getMaxDisplayRefreshRate();
    if (mounted) {
      setState(() {
        _isAdbAvailable = available;
        _maxRefreshRate = refreshRate;
        if (available) {
          _useAdbMode = true;
          _hostService.useAdbInputBridge = true;
        }
      });
    }
  }

  Future<void> _openAccessibilitySettings() async {
    await _hostService.openAccessibilitySettings();
    if (mounted) {
      await _checkAccessibility();
    }
  }

  Future<void> _toggleBroadcasting() async {
    if (_isStartingOrStopping) return;

    setState(() {
      _isStartingOrStopping = true;
    });

    try {
      if (_hostService.isRunning) {
        await _hostService.stopCapture();
        await _hostService.stop();
      } else {
        await _hostService.start(
          port: widget.port,
          deviceId: widget.deviceId,
          deviceName: widget.deviceName,
        );
        final targetFps = _useAdbMode
            ? _maxRefreshRate.toInt().clamp(30, 120)
            : 30;
        await _hostService.startCapture(fps: targetFps);
      }
    } finally {
      if (mounted) {
        setState(() {
          _isStartingOrStopping = false;
        });
      }
    }
  }

  String _format6Digit(String pin) {
    final clean = pin.replaceAll(' ', '');
    if (clean.length == 6) {
      return '${clean.substring(0, 3)} ${clean.substring(3)}';
    }
    return pin;
  }

  @override
  Widget build(BuildContext context) {
    final isRunning = _hostService.isRunning;
    final currentPin = _hostService.currentPin;

    return AlertDialog(
      backgroundColor: NeobrutalTheme.paper,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(NeobrutalTheme.radius),
        side: const BorderSide(
          color: NeobrutalTheme.ink,
          width: NeobrutalTheme.borderWidth,
        ),
      ),
      title: Row(
        children: [
          Icon(
            isRunning ? Icons.screen_share : Icons.mobile_screen_share,
            color: NeobrutalTheme.ink,
          ),
          const SizedBox(width: 8),
          const Expanded(
            child: Text(
              'Broadcast Screen (Host)',
              style: TextStyle(
                color: NeobrutalTheme.ink,
                fontSize: 18,
                fontWeight: FontWeight.w900,
              ),
            ),
          ),
          IconButton(
            icon: const Icon(Icons.close, color: NeobrutalTheme.ink),
            onPressed: () => Navigator.of(context).pop(),
          ),
        ],
      ),
      content: SingleChildScrollView(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            // Accessibility warning card if disabled
            if (!_isAccessibilityEnabled) ...[
              Card(
                key: const Key('accessibility_warning_card'),
                color: NeobrutalTheme.yellow,
                elevation: 0,
                shape: RoundedRectangleBorder(
                  side: const BorderSide(
                    color: NeobrutalTheme.ink,
                    width: NeobrutalTheme.compactBorderWidth,
                  ),
                  borderRadius: BorderRadius.circular(NeobrutalTheme.radius),
                ),
                child: Padding(
                  padding: const EdgeInsets.all(12.0),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      const Row(
                        children: [
                          Icon(
                            Icons.warning_amber_rounded,
                            color: NeobrutalTheme.ink,
                          ),
                          SizedBox(width: 8),
                          Text(
                            'Accessibility Permission Needed',
                            style: TextStyle(
                              color: NeobrutalTheme.ink,
                              fontWeight: FontWeight.w900,
                              fontSize: 13,
                            ),
                          ),
                        ],
                      ),
                      const SizedBox(height: 6),
                      const Text(
                        'To allow remote control gestures (touch, swipe, back, home) from PC, enable the VrV Desk Accessibility Service.',
                        style: TextStyle(
                          color: NeobrutalTheme.ink,
                          fontSize: 12,
                          fontWeight: FontWeight.w600,
                        ),
                      ),
                      const SizedBox(height: 8),
                      SizedBox(
                        width: double.infinity,
                        child: OutlinedButton.icon(
                          key: const Key('open_accessibility_settings_button'),
                          style: OutlinedButton.styleFrom(
                            backgroundColor: NeobrutalTheme.surface,
                            foregroundColor: NeobrutalTheme.ink,
                            side: const BorderSide(
                              color: NeobrutalTheme.ink,
                              width: NeobrutalTheme.compactBorderWidth,
                            ),
                          ),
                          onPressed: _openAccessibilitySettings,
                          icon: const Icon(Icons.settings, size: 16),
                          label: const Text('Open Accessibility Settings'),
                        ),
                      ),
                    ],
                  ),
                ),
              ),
              const SizedBox(height: 12),
            ],

            // Audio capability notification: fallback warning on < Android 10 or active badge
            if (!_isAudioSupported) ...[
              Card(
                key: const Key('audio_fallback_warning_card'),
                color: NeobrutalTheme.surface,
                elevation: 0,
                shape: RoundedRectangleBorder(
                  side: const BorderSide(
                    color: NeobrutalTheme.ink,
                    width: NeobrutalTheme.compactBorderWidth,
                  ),
                  borderRadius: BorderRadius.circular(NeobrutalTheme.radius),
                ),
                child: const Padding(
                  padding: EdgeInsets.all(12.0),
                  child: Row(
                    children: [
                      Icon(
                        Icons.volume_off,
                        color: NeobrutalTheme.ink,
                        size: 20,
                      ),
                      SizedBox(width: 8),
                      Expanded(
                        child: Text(
                          'Internal audio capture requires Android 10+. Video-only streaming active.',
                          style: TextStyle(
                            color: NeobrutalTheme.ink,
                            fontSize: 12,
                            fontWeight: FontWeight.w600,
                          ),
                        ),
                      ),
                    ],
                  ),
                ),
              ),
              const SizedBox(height: 12),
            ] else ...[
              Container(
                key: const Key('audio_active_badge'),
                padding: const EdgeInsets.symmetric(
                  horizontal: 10,
                  vertical: 6,
                ),
                margin: const EdgeInsets.only(bottom: 12),
                decoration: NeobrutalTheme.compactPanel(
                  color: NeobrutalTheme.mint,
                  shadow: false,
                ),
                child: const Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Icon(Icons.volume_up, color: NeobrutalTheme.ink, size: 16),
                    SizedBox(width: 6),
                    Text(
                      'Internal Audio (48kHz Stereo) Supported',
                      style: TextStyle(
                        color: NeobrutalTheme.ink,
                        fontSize: 11,
                        fontWeight: FontWeight.w800,
                      ),
                    ),
                  ],
                ),
              ),
            ],

            // High-Performance ADB Mode Card
            if (_isAdbAvailable) ...[
              Container(
                key: const Key('adb_high_perf_card'),
                margin: const EdgeInsets.only(bottom: 12),
                padding: const EdgeInsets.all(12),
                decoration: NeobrutalTheme.compactPanel(
                  color: NeobrutalTheme.cyan,
                  shadow: true,
                ),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Row(
                      children: [
                        const Icon(
                          Icons.flash_on,
                          color: NeobrutalTheme.ink,
                          size: 20,
                        ),
                        const SizedBox(width: 8),
                        Expanded(
                          child: Text(
                            'High-Performance ADB Mode (${_maxRefreshRate.toInt()}Hz)',
                            style: const TextStyle(
                              color: NeobrutalTheme.ink,
                              fontWeight: FontWeight.w900,
                              fontSize: 13,
                            ),
                          ),
                        ),
                        Switch(
                          key: const Key('adb_mode_switch'),
                          value: _useAdbMode,
                          activeThumbColor: NeobrutalTheme.yellow,
                          activeTrackColor: NeobrutalTheme.ink,
                          inactiveThumbColor: NeobrutalTheme.surface,
                          inactiveTrackColor: NeobrutalTheme.paper,
                          onChanged: (val) {
                            setState(() {
                              _useAdbMode = val;
                              _hostService.useAdbInputBridge = val;
                            });
                          },
                        ),
                      ],
                    ),
                    const SizedBox(height: 4),
                    Text(
                      _useAdbMode
                          ? 'Ultra-low latency direct shell/uinput input active. Target ${_maxRefreshRate.toInt()}Hz.'
                          : 'Using standard AccessibilityService gestures.',
                      style: const TextStyle(
                        color: NeobrutalTheme.ink,
                        fontSize: 11,
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                  ],
                ),
              ),
            ],

            // Status Card
            Container(
              padding: const EdgeInsets.all(12),
              decoration: NeobrutalTheme.compactPanel(
                color: isRunning ? NeobrutalTheme.mint : NeobrutalTheme.surface,
                shadow: true,
              ),
              child: Row(
                children: [
                  Container(
                    width: 12,
                    height: 12,
                    decoration: BoxDecoration(
                      shape: BoxShape.circle,
                      color: isRunning
                          ? NeobrutalTheme.mint
                          : NeobrutalTheme.muted,
                      border: Border.all(color: NeobrutalTheme.ink, width: 2),
                    ),
                  ),
                  const SizedBox(width: 10),
                  Text(
                    isRunning ? 'Broadcasting Active' : 'Broadcasting Inactive',
                    style: const TextStyle(
                      color: NeobrutalTheme.ink,
                      fontWeight: FontWeight.w900,
                      fontSize: 14,
                    ),
                  ),
                ],
              ),
            ),
            const SizedBox(height: 16),

            // Active broadcast details (PIN, QR Code, metrics)
            if (isRunning) ...[
              if (currentPin.isNotEmpty) ...[
                Container(
                  key: const Key('host_pin_display'),
                  padding: const EdgeInsets.all(12),
                  decoration: NeobrutalTheme.panel(
                    color: NeobrutalTheme.yellow,
                  ),
                  child: Column(
                    children: [
                      const Text(
                        'One-Time Connection PIN',
                        style: TextStyle(
                          color: NeobrutalTheme.ink,
                          fontSize: 12,
                          fontWeight: FontWeight.w700,
                        ),
                      ),
                      const SizedBox(height: 4),
                      Text(
                        _format6Digit(currentPin),
                        style: const TextStyle(
                          color: NeobrutalTheme.ink,
                          fontSize: 26,
                          fontWeight: FontWeight.w900,
                          letterSpacing: 3,
                        ),
                      ),
                    ],
                  ),
                ),
                const SizedBox(height: 16),
                Center(
                  child: Container(
                    width: 180,
                    height: 180,
                    padding: const EdgeInsets.all(10),
                    decoration: NeobrutalTheme.panel(
                      color: NeobrutalTheme.surface,
                    ),
                    child: QrImageView(
                      key: const Key('host_qr_image_view'),
                      data: QrPairingService.serialize(
                        QrPairingData(
                          deviceId: widget.deviceId,
                          pin: currentPin,
                          port: widget.port,
                        ),
                      ),
                      version: QrVersions.auto,
                      size: 160.0,
                    ),
                  ),
                ),
                const SizedBox(height: 16),
              ],

              // Metrics: frames sent, connected clients
              Container(
                padding: const EdgeInsets.symmetric(vertical: 8),
                decoration: NeobrutalTheme.compactPanel(
                  color: NeobrutalTheme.surface,
                ),
                child: Row(
                  mainAxisAlignment: MainAxisAlignment.spaceEvenly,
                  children: [
                    Column(
                      children: [
                        const Text(
                          'Frames Sent',
                          style: TextStyle(
                            color: NeobrutalTheme.muted,
                            fontSize: 11,
                            fontWeight: FontWeight.w700,
                          ),
                        ),
                        const SizedBox(height: 2),
                        Text(
                          '${_hostService.framesSent}',
                          key: const Key('host_frames_sent_text'),
                          style: const TextStyle(
                            color: NeobrutalTheme.ink,
                            fontSize: 16,
                            fontWeight: FontWeight.w900,
                          ),
                        ),
                      ],
                    ),
                    Container(width: 2, height: 28, color: NeobrutalTheme.ink),
                    Column(
                      children: [
                        const Text(
                          'Clients',
                          style: TextStyle(
                            color: NeobrutalTheme.muted,
                            fontSize: 11,
                            fontWeight: FontWeight.w700,
                          ),
                        ),
                        const SizedBox(height: 2),
                        Text(
                          '${_hostService.authenticatedClientsCount} connected',
                          key: const Key('host_clients_count_text'),
                          style: const TextStyle(
                            color: NeobrutalTheme.ink,
                            fontSize: 16,
                            fontWeight: FontWeight.w900,
                          ),
                        ),
                      ],
                    ),
                  ],
                ),
              ),
              const SizedBox(height: 16),
            ],

            // Toggle Button
            ElevatedButton.icon(
              key: const Key('host_mode_toggle_button'),
              style: ElevatedButton.styleFrom(
                backgroundColor: isRunning
                    ? NeobrutalTheme.danger
                    : NeobrutalTheme.mint,
                foregroundColor: NeobrutalTheme.ink,
                padding: const EdgeInsets.symmetric(vertical: 12),
                shape: RoundedRectangleBorder(
                  borderRadius: BorderRadius.circular(NeobrutalTheme.radius),
                  side: const BorderSide(
                    color: NeobrutalTheme.ink,
                    width: NeobrutalTheme.borderWidth,
                  ),
                ),
              ),
              onPressed: _isStartingOrStopping ? null : _toggleBroadcasting,
              icon: _isStartingOrStopping
                  ? const SizedBox(
                      width: 16,
                      height: 16,
                      child: CircularProgressIndicator(
                        strokeWidth: 2,
                        valueColor: AlwaysStoppedAnimation<Color>(
                          NeobrutalTheme.ink,
                        ),
                      ),
                    )
                  : Icon(isRunning ? Icons.stop : Icons.play_arrow),
              label: Text(
                isRunning ? 'Stop Broadcasting' : 'Start Broadcasting',
                style: const TextStyle(fontWeight: FontWeight.w900),
              ),
            ),
          ],
        ),
      ),
    );
  }
}
