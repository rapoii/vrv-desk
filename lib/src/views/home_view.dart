import 'dart:async';
import 'dart:io' show Platform;
import 'package:flutter/foundation.dart' show kIsWeb;
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import '../models/device.dart';
import '../services/android_host_service.dart';
import '../services/lan_discovery_service.dart';
import '../widgets/host_mode_dialog.dart';
import '../widgets/pin_dialog.dart';
import '../widgets/qr_code_dialog.dart';
import '../services/qr_pairing_service.dart';
import 'qr_scanner_view.dart';
import 'mirror_view.dart';

class HomeView extends StatefulWidget {
  final String myDeviceId;
  final List<DiscoveredDevice> initialDevices;
  final void Function(DiscoveredDevice device, String pin)? onConnect;
  final String signalingUrl;
  final LanDiscoveryService? lanDiscoveryService;
  final AndroidHostService? androidHostService;
  final bool? isAndroidOverride;

  const HomeView({
    super.key,
    required this.myDeviceId,
    this.initialDevices = const [],
    this.onConnect,
    this.signalingUrl = 'ws://10.0.2.2:53212',
    this.lanDiscoveryService,
    this.androidHostService,
    this.isAndroidOverride,
  });

  static bool is6DigitDeviceId(String input) {
    final cleaned = input.replaceAll(' ', '');
    return RegExp(r'^\d{6}$').hasMatch(cleaned);
  }

  @override
  State<HomeView> createState() => _HomeViewState();
}

class _HomeViewState extends State<HomeView> {
  late List<DiscoveredDevice> _devices;
  bool _isCopied = false;
  final TextEditingController _quickConnectController = TextEditingController();
  late final LanDiscoveryService _lanService;
  StreamSubscription<List<DiscoveredDevice>>? _lanSubscription;

  @override
  void initState() {
    super.initState();
    _devices = List.from(widget.initialDevices);
    _lanService = widget.lanDiscoveryService ?? LanDiscoveryService();
    _lanService.start();
    _lanSubscription = _lanService.devicesStream.listen((devices) {
      if (mounted) {
        setState(() {
          _devices = devices;
        });
      }
    });
  }

  @override
  void dispose() {
    _lanSubscription?.cancel();
    _lanService.stop();
    _quickConnectController.dispose();
    super.dispose();
  }

  @override
  void didUpdateWidget(covariant HomeView oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.initialDevices != widget.initialDevices) {
      _devices = List.from(widget.initialDevices);
    }
  }

  bool get _isAndroid =>
      widget.isAndroidOverride ?? (!kIsWeb && Platform.isAndroid);

  void _showHostModeDialog() {
    showDialog(
      context: context,
      builder: (_) => HostModeDialog(
        hostService: widget.androidHostService,
        deviceId: widget.myDeviceId,
      ),
    );
  }

  void _copyDeviceId() {
    Clipboard.setData(ClipboardData(text: widget.myDeviceId));
    setState(() {
      _isCopied = true;
    });
    ScaffoldMessenger.of(context).showSnackBar(
      const SnackBar(
        content: Text('Device ID copied to clipboard'),
        duration: Duration(seconds: 2),
      ),
    );
  }

  void _showMyQrCode() {
    showDialog(
      context: context,
      builder: (_) => QrCodeDialog(
        pairingData: QrPairingData(
          deviceId: widget.myDeviceId,
          signalingUrl: widget.signalingUrl,
        ),
      ),
    );
  }

  Future<void> _openQrScanner() async {
    final result = await Navigator.of(context).push<QrPairingData>(
      MaterialPageRoute(
        builder: (_) => const QrScannerView(),
      ),
    );

    if (result != null && mounted) {
      if (result.ipAddress != null && result.ipAddress!.isNotEmpty) {
        Navigator.of(context).push(
          MaterialPageRoute(
            builder: (_) => MirrorView(
              hostIp: result.ipAddress,
              port: result.port,
              initialPin: result.pin,
            ),
          ),
        );
      } else if (result.deviceId.isNotEmpty) {
        Navigator.of(context).push(
          MaterialPageRoute(
            builder: (_) => MirrorView(
              signalingUrl: result.signalingUrl ?? widget.signalingUrl,
              targetDeviceId: result.deviceId,
              initialPin: result.pin,
            ),
          ),
        );
      }
    }
  }

  void _showPinDialog(DiscoveredDevice device) {
    showDialog(
      context: context,
      builder: (context) => PinDialog(
        deviceName: device.deviceName,
        onSubmitted: (pin) {
          if (widget.onConnect != null) {
            widget.onConnect!(device, pin);
          } else {
            Navigator.of(context).push(
              MaterialPageRoute(
                builder: (_) => MirrorView(
                  hostIp: device.ipAddress,
                  port: device.port,
                  initialPin: pin,
                ),
              ),
            );
          }
        },
      ),
    );
  }

  void _handleConnectInput(String input, String pin) {
    final clean = input.trim();
    if (clean.isEmpty) return;

    final cleanPin = pin.trim().isNotEmpty ? pin.trim() : null;

    if (HomeView.is6DigitDeviceId(clean)) {
      final targetId = clean.replaceAll(' ', '');
      Navigator.of(context).push(
        MaterialPageRoute(
          builder: (_) => MirrorView(
            signalingUrl: widget.signalingUrl,
            targetDeviceId: targetId,
            initialPin: cleanPin,
          ),
        ),
      );
    } else {
      Navigator.of(context).push(
        MaterialPageRoute(
          builder: (_) => MirrorView(
            hostIp: clean,
            port: 53211,
            initialPin: cleanPin,
          ),
        ),
      );
    }
  }

  void _showDirectIpDialog() {
    final inputController = TextEditingController();
    final pinController = TextEditingController();
    showDialog(
      context: context,
      builder: (dialogContext) => AlertDialog(
        title: const Text('Connect to PC / Host'),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            const Text(
              'Enter 6-digit Device ID (e.g. 849 201) or Host IP address:',
              style: TextStyle(fontSize: 13, color: Colors.grey),
            ),
            const SizedBox(height: 12),
            TextField(
              key: const Key('direct_ip_field'),
              controller: inputController,
              decoration: const InputDecoration(
                labelText: 'Device ID or Host IP',
                border: OutlineInputBorder(),
                hintText: '849 201 or 192.168.x.x',
              ),
              keyboardType: TextInputType.text,
            ),
            const SizedBox(height: 12),
            TextField(
              key: const Key('direct_pin_field'),
              controller: pinController,
              decoration: const InputDecoration(
                labelText: 'Host PIN (Optional, 6 digits)',
                border: OutlineInputBorder(),
                hintText: 'e.g. 123456',
              ),
              keyboardType: TextInputType.number,
              maxLength: 6,
            ),
          ],
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(dialogContext).pop(),
            child: const Text('Cancel'),
          ),
          ElevatedButton(
            key: const Key('dialog_connect_button'),
            onPressed: () {
              final text = inputController.text.trim();
              final pin = pinController.text.trim();
              if (text.isNotEmpty) {
                Navigator.of(dialogContext).pop();
                _handleConnectInput(text, pin);
              }
            },
            child: const Text('Connect'),
          ),
        ],
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('Mirror & Remote Control'),
        actions: [
          IconButton(
            key: const Key('scan_qr_appbar_button'),
            icon: const Icon(Icons.qr_code_scanner),
            tooltip: 'Scan QR Code',
            onPressed: _openQrScanner,
          ),
          IconButton(
            key: const Key('direct_connect_button'),
            icon: const Icon(Icons.cast),
            tooltip: 'Connect to Host IP',
            onPressed: _showDirectIpDialog,
          ),
        ],
      ),

      body: SingleChildScrollView(
        padding: const EdgeInsets.all(16.0),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Card(
              elevation: 2,
              shape: RoundedRectangleBorder(
                borderRadius: BorderRadius.circular(12),
              ),
              child: Padding(
                padding: const EdgeInsets.all(16.0),
                child: Column(
                  children: [
                    const Text(
                      'Your Device ID',
                      style: TextStyle(
                        fontSize: 14,
                        color: Colors.grey,
                        fontWeight: FontWeight.w500,
                      ),
                    ),
                    const SizedBox(height: 8),
                    Text(
                      widget.myDeviceId,
                      key: const Key('my_device_id_text'),
                      style: const TextStyle(
                        fontSize: 28,
                        fontWeight: FontWeight.bold,
                        letterSpacing: 4,
                      ),
                    ),
                    const SizedBox(height: 12),
                    Row(
                      mainAxisAlignment: MainAxisAlignment.center,
                      children: [
                        ElevatedButton.icon(
                          key: const Key('copy_device_id_button'),
                          onPressed: _copyDeviceId,
                          icon: Icon(_isCopied ? Icons.check : Icons.copy),
                          label: Text(_isCopied ? 'Copied' : 'Copy ID'),
                        ),
                        const SizedBox(width: 12),
                        OutlinedButton.icon(
                          key: const Key('show_qr_button'),
                          onPressed: _showMyQrCode,
                          icon: const Icon(Icons.qr_code_2),
                          label: const Text('Show QR'),
                        ),
                      ],
                    ),
                    if (_isAndroid) ...[
                      const SizedBox(height: 12),
                      SizedBox(
                        width: double.infinity,
                        child: FilledButton.icon(
                          key: const Key('share_screen_button'),
                          style: FilledButton.styleFrom(
                            backgroundColor: Colors.teal.shade700,
                            foregroundColor: Colors.white,
                          ),
                          onPressed: _showHostModeDialog,
                          icon: const Icon(Icons.screen_share),
                          label: const Text(
                            'Share My Screen (Host)',
                            style: TextStyle(fontWeight: FontWeight.bold),
                          ),
                        ),
                      ),
                    ],
                  ],
                ),
              ),
            ),
            const SizedBox(height: 16),
            Card(
              elevation: 2,
              shape: RoundedRectangleBorder(
                borderRadius: BorderRadius.circular(12),
              ),
              child: Padding(
                padding: const EdgeInsets.all(16.0),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: [
                    Row(
                      children: [
                        Icon(
                          Icons.bolt,
                          color: Theme.of(context).colorScheme.primary,
                        ),
                        const SizedBox(width: 8),
                        const Text(
                          'Quick Connect',
                          style: TextStyle(
                            fontSize: 16,
                            fontWeight: FontWeight.bold,
                          ),
                        ),
                      ],
                    ),
                    const SizedBox(height: 8),
                    const Text(
                      'Connect to any remote PC using 6-digit Device ID or local IP address:',
                      style: TextStyle(fontSize: 13, color: Colors.grey),
                    ),
                    const SizedBox(height: 12),
                    TextField(
                      key: const Key('quick_connect_field'),
                      controller: _quickConnectController,
                      decoration: const InputDecoration(
                        labelText: 'Remote Device ID or IP',
                        border: OutlineInputBorder(),
                        hintText: 'e.g. 849 201 or 192.168.1.100',
                        prefixIcon: Icon(Icons.cast_connected),
                      ),
                      keyboardType: TextInputType.text,
                    ),
                    const SizedBox(height: 10),
                    Row(
                      children: [
                        Expanded(
                          child: ElevatedButton.icon(
                            key: const Key('quick_connect_button'),
                            onPressed: () {
                              _handleConnectInput(_quickConnectController.text, '');
                            },
                            icon: const Icon(Icons.arrow_forward),
                            label: const Text('Connect to Device'),
                          ),
                        ),
                        const SizedBox(width: 8),
                        IconButton.filledTonal(
                          key: const Key('scan_qr_quick_button'),
                          tooltip: 'Scan QR Code',
                          icon: const Icon(Icons.qr_code_scanner),
                          onPressed: _openQrScanner,
                        ),
                      ],
                    ),
                  ],
                ),
              ),
            ),
            const SizedBox(height: 24),
            Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                const Text(
                  'Discovered Devices',
                  style: TextStyle(
                    fontSize: 18,
                    fontWeight: FontWeight.bold,
                  ),
                ),
                Text(
                  '${_devices.length} found',
                  key: const Key('discovered_count_text'),
                  style: const TextStyle(color: Colors.grey),
                ),
              ],
            ),
            const SizedBox(height: 12),
            if (_devices.isEmpty)
              const Padding(
                padding: EdgeInsets.symmetric(vertical: 32.0),
                child: Center(
                  child: Column(
                    children: [
                      CircularProgressIndicator(),
                      SizedBox(height: 16),
                      Text(
                        'Scanning for devices on local network...',
                        style: TextStyle(color: Colors.grey),
                      ),
                    ],
                  ),
                ),
              )
            else
              ListView.separated(
                shrinkWrap: true,
                physics: const NeverScrollableScrollPhysics(),
                itemCount: _devices.length,
                separatorBuilder: (context, index) => const Divider(),
                itemBuilder: (context, index) {
                  final dev = _devices[index];
                  IconData iconData = Icons.devices;
                  if (dev.osType.toLowerCase().contains('windows')) {
                    iconData = Icons.desktop_windows;
                  } else if (dev.osType.toLowerCase().contains('android')) {
                    iconData = Icons.phone_android;
                  }

                  return ListTile(
                    key: Key('device_tile_${dev.deviceId}'),
                    leading: CircleAvatar(
                      child: Icon(iconData),
                    ),
                    title: Text(
                      dev.deviceName,
                      style: const TextStyle(fontWeight: FontWeight.bold),
                    ),
                    subtitle: Text('${dev.ipAddress}:${dev.port} • ${dev.osType}'),
                    trailing: ElevatedButton(
                      key: Key('connect_button_${dev.deviceId}'),
                      onPressed: () => _showPinDialog(dev),
                      child: const Text('Connect'),
                    ),
                  );
                },
              ),
          ],
        ),
      ),
    );
  }
}
