import 'dart:async';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:qr_flutter/qr_flutter.dart';
import '../services/android_host_service.dart';
import '../services/qr_pairing_service.dart';

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
  bool _isStartingOrStopping = false;
  Timer? _metricsTimer;

  @override
  void initState() {
    super.initState();
    _hostService = widget.hostService ?? AndroidHostService();
    _checkAccessibility();
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
        await _hostService.startCapture();
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
      backgroundColor: const Color(0xFF1E1E2E),
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(16)),
      title: Row(
        children: [
          Icon(
            isRunning ? Icons.screen_share : Icons.mobile_screen_share,
            color: isRunning ? Colors.greenAccent : Colors.blueAccent,
          ),
          const SizedBox(width: 8),
          const Expanded(
            child: Text(
              'Broadcast Screen (Host)',
              style: TextStyle(
                color: Colors.white,
                fontSize: 18,
                fontWeight: FontWeight.bold,
              ),
            ),
          ),
          IconButton(
            icon: const Icon(Icons.close, color: Colors.white70),
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
                color: Colors.amber.withOpacity(0.15),
                shape: RoundedRectangleBorder(
                  side: const BorderSide(color: Colors.amber, width: 1),
                  borderRadius: BorderRadius.circular(8),
                ),
                child: Padding(
                  padding: const EdgeInsets.all(12.0),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      const Row(
                        children: [
                          Icon(Icons.warning_amber_rounded, color: Colors.amber),
                          SizedBox(width: 8),
                          Text(
                            'Accessibility Permission Needed',
                            style: TextStyle(
                              color: Colors.amber,
                              fontWeight: FontWeight.bold,
                              fontSize: 13,
                            ),
                          ),
                        ],
                      ),
                      const SizedBox(height: 6),
                      const Text(
                        'To allow remote control gestures (touch, swipe, back, home) from PC, enable the VrV Desk Accessibility Service.',
                        style: TextStyle(color: Colors.white70, fontSize: 12),
                      ),
                      const SizedBox(height: 8),
                      SizedBox(
                        width: double.infinity,
                        child: OutlinedButton.icon(
                          key: const Key('open_accessibility_settings_button'),
                          style: OutlinedButton.styleFrom(
                            foregroundColor: Colors.amber,
                            side: const BorderSide(color: Colors.amber),
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

            // Status Card
            Container(
              padding: const EdgeInsets.all(12),
              decoration: BoxDecoration(
                color: isRunning
                    ? Colors.greenAccent.withOpacity(0.1)
                    : Colors.white.withOpacity(0.05),
                borderRadius: BorderRadius.circular(10),
                border: Border.all(
                  color: isRunning
                      ? Colors.greenAccent.withOpacity(0.4)
                      : Colors.white12,
                ),
              ),
              child: Row(
                children: [
                  Container(
                    width: 12,
                    height: 12,
                    decoration: BoxDecoration(
                      shape: BoxShape.circle,
                      color: isRunning ? Colors.greenAccent : Colors.grey,
                    ),
                  ),
                  const SizedBox(width: 10),
                  Text(
                    isRunning ? 'Broadcasting Active' : 'Broadcasting Inactive',
                    style: TextStyle(
                      color: isRunning ? Colors.greenAccent : Colors.white70,
                      fontWeight: FontWeight.bold,
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
                  decoration: BoxDecoration(
                    color: Colors.blueAccent.withOpacity(0.15),
                    borderRadius: BorderRadius.circular(8),
                    border: Border.all(color: Colors.blueAccent.withOpacity(0.4)),
                  ),
                  child: Column(
                    children: [
                      const Text(
                        'One-Time Connection PIN',
                        style: TextStyle(color: Colors.white70, fontSize: 12),
                      ),
                      const SizedBox(height: 4),
                      Text(
                        _format6Digit(currentPin),
                        style: const TextStyle(
                          color: Colors.blueAccent,
                          fontSize: 26,
                          fontWeight: FontWeight.bold,
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
                    decoration: BoxDecoration(
                      color: Colors.white,
                      borderRadius: BorderRadius.circular(12),
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
              Row(
                mainAxisAlignment: MainAxisAlignment.spaceEvenly,
                children: [
                  Column(
                    children: [
                      const Text(
                        'Frames Sent',
                        style: TextStyle(color: Colors.white54, fontSize: 11),
                      ),
                      const SizedBox(height: 2),
                      Text(
                        '${_hostService.framesSent}',
                        key: const Key('host_frames_sent_text'),
                        style: const TextStyle(
                          color: Colors.white,
                          fontSize: 16,
                          fontWeight: FontWeight.bold,
                        ),
                      ),
                    ],
                  ),
                  Container(width: 1, height: 28, color: Colors.white12),
                  Column(
                    children: [
                      const Text(
                        'Clients',
                        style: TextStyle(color: Colors.white54, fontSize: 11),
                      ),
                      const SizedBox(height: 2),
                      Text(
                        '${_hostService.authenticatedClientsCount} connected',
                        key: const Key('host_clients_count_text'),
                        style: const TextStyle(
                          color: Colors.white,
                          fontSize: 16,
                          fontWeight: FontWeight.bold,
                        ),
                      ),
                    ],
                  ),
                ],
              ),
              const SizedBox(height: 16),
            ],

            // Toggle Button
            ElevatedButton.icon(
              key: const Key('host_mode_toggle_button'),
              style: ElevatedButton.styleFrom(
                backgroundColor: isRunning ? Colors.redAccent : Colors.blueAccent,
                foregroundColor: Colors.white,
                padding: const EdgeInsets.symmetric(vertical: 12),
                shape: RoundedRectangleBorder(
                  borderRadius: BorderRadius.circular(8),
                ),
              ),
              onPressed: _isStartingOrStopping ? null : _toggleBroadcasting,
              icon: _isStartingOrStopping
                  ? const SizedBox(
                      width: 16,
                      height: 16,
                      child: CircularProgressIndicator(
                        strokeWidth: 2,
                        valueColor: AlwaysStoppedAnimation<Color>(Colors.white),
                      ),
                    )
                  : Icon(isRunning ? Icons.stop : Icons.play_arrow),
              label: Text(
                isRunning ? 'Stop Broadcasting' : 'Start Broadcasting',
                style: const TextStyle(fontWeight: FontWeight.bold),
              ),
            ),
          ],
        ),
      ),
    );
  }
}
