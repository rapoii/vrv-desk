import 'dart:async';
import 'dart:io' show Platform;

import 'package:flutter/foundation.dart' show kIsWeb;
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../models/device.dart';
import '../services/android_host_service.dart';
import '../services/lan_discovery_service.dart';
import '../services/qr_pairing_service.dart';
import '../services/unattended_storage.dart';
import '../theme/neobrutalist_theme.dart';
import '../widgets/host_mode_dialog.dart';
import '../widgets/pin_dialog.dart';
import '../widgets/qr_code_dialog.dart';
import 'mirror_view.dart';
import 'qr_scanner_view.dart';

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

class _HomeViewState extends State<HomeView>
    with SingleTickerProviderStateMixin {
  late List<DiscoveredDevice> _devices;
  bool _isCopied = false;
  final TextEditingController _quickConnectController = TextEditingController();
  late final LanDiscoveryService _lanService;
  StreamSubscription<List<DiscoveredDevice>>? _lanSubscription;
  late final AnimationController _entranceController;

  @override
  void initState() {
    super.initState();
    _entranceController = AnimationController(
      vsync: this,
      duration: const Duration(milliseconds: 350),
    )..forward();
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
    _entranceController.dispose();
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
      MaterialPageRoute(builder: (_) => const QrScannerView()),
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
    final savedPassword =
        UnattendedStorage.getSavedPassword(device.deviceId) ??
        UnattendedStorage.getSavedPassword(device.ipAddress);

    showDialog(
      context: context,
      builder: (context) => PinDialog(
        deviceName: device.deviceName,
        initialSecret: savedPassword,
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
        onSubmittedWithRemember: (secret, remember) {
          if (remember) {
            UnattendedStorage.savePassword(device.deviceId, secret);
            UnattendedStorage.savePassword(device.ipAddress, secret);
          } else if (savedPassword != null) {
            UnattendedStorage.removePassword(device.deviceId);
            UnattendedStorage.removePassword(device.ipAddress);
          }
          if (mounted) setState(() {});
        },
      ),
    );
  }

  void _handleConnectInput(String input, String pin) {
    final clean = input.trim();
    if (clean.isEmpty) return;

    final targetKey = clean.replaceAll(' ', '');
    final savedPassword = UnattendedStorage.getSavedPassword(targetKey);
    final cleanPin = pin.trim().isNotEmpty ? pin.trim() : savedPassword;

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
          builder: (_) =>
              MirrorView(hostIp: clean, port: 53211, initialPin: cleanPin),
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
              style: TextStyle(
                fontSize: 13,
                color: NeobrutalTheme.muted,
                fontWeight: FontWeight.w600,
              ),
            ),
            const SizedBox(height: 12),
            TextField(
              key: const Key('direct_ip_field'),
              controller: inputController,
              decoration: const InputDecoration(
                labelText: 'Device ID or Host IP',
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
                hintText: 'e.g. 123456',
              ),
              keyboardType: TextInputType.number,
              maxLength: 6,
            ),
          ],
        ),
        actions: [
          NeobrutalPressable(
            cornerRadius: 4,
            shadowOffset: const Offset(2, 2),
            child: TextButton(
              onPressed: () => Navigator.of(dialogContext).pop(),
              child: const Text('Cancel'),
            ),
          ),
          NeobrutalPressable(
            cornerRadius: 6,
            shadowOffset: const Offset(3, 3),
            child: ElevatedButton(
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
          NeobrutalPressable(
            cornerRadius: 4,
            shadowOffset: const Offset(2, 2),
            child: IconButton(
              key: const Key('scan_qr_appbar_button'),
              icon: const Icon(Icons.qr_code_scanner),
              tooltip: 'Scan QR Code',
              onPressed: _openQrScanner,
            ),
          ),
          const SizedBox(width: 8),
          NeobrutalPressable(
            cornerRadius: 4,
            shadowOffset: const Offset(2, 2),
            child: IconButton(
              key: const Key('direct_connect_button'),
              icon: const Icon(Icons.cast),
              tooltip: 'Connect to Host IP',
              onPressed: _showDirectIpDialog,
            ),
          ),
          const SizedBox(width: 10),
        ],
      ),
      body: SingleChildScrollView(
        padding: const EdgeInsets.symmetric(horizontal: 14.0, vertical: 8.0),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            StaggeredCardEntry(
              animation: _entranceController,
              startInterval: 0.0,
              endInterval: 0.6,
              child: Container(
                decoration: NeobrutalTheme.panel(
                  color: NeobrutalTheme.surface,
                  shadow: true,
                ),
                padding: const EdgeInsets.all(12.0),
                child: Column(
                  children: [
                    const Text(
                      'Your Device ID',
                      style: TextStyle(
                        fontSize: 13,
                        color: NeobrutalTheme.ink,
                        fontWeight: FontWeight.w900,
                        letterSpacing: 1.1,
                      ),
                    ),
                    const SizedBox(height: 6),
                    Container(
                      padding: const EdgeInsets.symmetric(
                        horizontal: 12,
                        vertical: 6,
                      ),
                      decoration: NeobrutalTheme.compactPanel(
                        color: NeobrutalTheme.yellow,
                        shadow: false,
                      ),
                      child: Text(
                        widget.myDeviceId,
                        key: const Key('my_device_id_text'),
                        style: const TextStyle(
                          fontSize: 24,
                          fontWeight: FontWeight.w900,
                          letterSpacing: 3,
                          color: NeobrutalTheme.ink,
                        ),
                      ),
                    ),
                    const SizedBox(height: 10),
                    Row(
                      mainAxisAlignment: MainAxisAlignment.center,
                      children: [
                        NeobrutalPressable(
                          shadowOffset: const Offset(3, 3),
                          child: ElevatedButton.icon(
                            key: const Key('copy_device_id_button'),
                            onPressed: _copyDeviceId,
                            icon: AnimatedSwitcher(
                              duration: const Duration(milliseconds: 200),
                              child: Icon(
                                _isCopied ? Icons.check : Icons.copy,
                                key: ValueKey(_isCopied),
                              ),
                            ),
                            label: AnimatedSwitcher(
                              duration: const Duration(milliseconds: 200),
                              child: Text(
                                _isCopied ? 'Copied' : 'Copy ID',
                                key: ValueKey(_isCopied),
                              ),
                            ),
                          ),
                        ),
                        const SizedBox(width: 10),
                        NeobrutalPressable(
                          shadowOffset: const Offset(3, 3),
                          child: OutlinedButton.icon(
                            key: const Key('show_qr_button'),
                            onPressed: _showMyQrCode,
                            icon: const Icon(Icons.qr_code_2),
                            label: const Text('Show QR'),
                          ),
                        ),
                      ],
                    ),
                    if (_isAndroid) ...[
                      const SizedBox(height: 10),
                      SizedBox(
                        width: double.infinity,
                        child: NeobrutalPressable(
                          shadowOffset: const Offset(3, 3),
                          child: FilledButton.icon(
                            key: const Key('share_screen_button'),
                            style: FilledButton.styleFrom(
                              backgroundColor: NeobrutalTheme.cyan,
                              foregroundColor: NeobrutalTheme.ink,
                            ),
                            onPressed: _showHostModeDialog,
                            icon: const Icon(Icons.screen_share),
                            label: const Text(
                              'Share My Screen (Host)',
                              style: TextStyle(fontWeight: FontWeight.w900),
                            ),
                          ),
                        ),
                      ),
                    ],
                  ],
                ),
              ),
            ),
            const SizedBox(height: 10),
            StaggeredCardEntry(
              animation: _entranceController,
              startInterval: 0.2,
              endInterval: 0.8,
              child: Container(
                decoration: NeobrutalTheme.panel(
                  color: NeobrutalTheme.surface,
                  shadow: true,
                ),
                padding: const EdgeInsets.all(12.0),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: [
                    const Row(
                      children: [
                        Icon(Icons.bolt, color: NeobrutalTheme.ink),
                        SizedBox(width: 8),
                        Text(
                          'Quick Connect',
                          style: TextStyle(
                            fontSize: 16,
                            fontWeight: FontWeight.w900,
                            color: NeobrutalTheme.ink,
                          ),
                        ),
                      ],
                    ),
                    const SizedBox(height: 6),
                    const Text(
                      'Connect to any remote PC using 6-digit Device ID or local IP address:',
                      style: TextStyle(
                        fontSize: 12,
                        color: NeobrutalTheme.muted,
                        fontWeight: FontWeight.w600,
                      ),
                    ),
                    const SizedBox(height: 10),
                    TextField(
                      key: const Key('quick_connect_field'),
                      controller: _quickConnectController,
                      decoration: const InputDecoration(
                        labelText: 'Remote Device ID or IP',
                        hintText: 'e.g. 849 201 or 192.168.1.100',
                        prefixIcon: Icon(Icons.cast_connected),
                      ),
                      keyboardType: TextInputType.text,
                    ),
                    const SizedBox(height: 8),
                    Row(
                      children: [
                        Expanded(
                          child: NeobrutalPressable(
                            shadowOffset: const Offset(3, 3),
                            child: ElevatedButton.icon(
                              key: const Key('quick_connect_button'),
                              onPressed: () {
                                _handleConnectInput(
                                  _quickConnectController.text,
                                  '',
                                );
                              },
                              icon: const Icon(Icons.arrow_forward),
                              label: const Text('Connect to Device'),
                            ),
                          ),
                        ),
                        const SizedBox(width: 8),
                        NeobrutalPressable(
                          cornerRadius: 4,
                          shadowOffset: const Offset(2, 2),
                          child: IconButton.filledTonal(
                            key: const Key('scan_qr_quick_button'),
                            tooltip: 'Scan QR Code',
                            icon: const Icon(Icons.qr_code_scanner),
                            onPressed: _openQrScanner,
                          ),
                        ),
                      ],
                    ),
                  ],
                ),
              ),
            ),
            const SizedBox(height: 12),
            StaggeredCardEntry(
              animation: _entranceController,
              startInterval: 0.4,
              endInterval: 1.0,
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  Row(
                    mainAxisAlignment: MainAxisAlignment.spaceBetween,
                    children: [
                      const Text(
                        'Discovered Devices',
                        style: TextStyle(
                          fontSize: 17,
                          fontWeight: FontWeight.w900,
                          color: NeobrutalTheme.ink,
                        ),
                      ),
                      Container(
                        padding: const EdgeInsets.symmetric(
                          horizontal: 8,
                          vertical: 3,
                        ),
                        decoration: NeobrutalTheme.compactPanel(
                          color: NeobrutalTheme.paper,
                          shadow: false,
                        ),
                        child: Text(
                          '${_devices.length} found',
                          key: const Key('discovered_count_text'),
                          style: const TextStyle(
                            color: NeobrutalTheme.ink,
                            fontWeight: FontWeight.w800,
                            fontSize: 12,
                          ),
                        ),
                      ),
                    ],
                  ),
                  const SizedBox(height: 8),
                  if (_devices.isEmpty)
                    Padding(
                      padding: const EdgeInsets.symmetric(vertical: 24.0),
                      child: Center(
                        child: Container(
                          padding: const EdgeInsets.all(16),
                          decoration: NeobrutalTheme.panel(
                            color: NeobrutalTheme.paper,
                            shadow: false,
                          ),
                          child: const Column(
                            children: [
                              CircularProgressIndicator(
                                strokeWidth: 3,
                                color: NeobrutalTheme.ink,
                              ),
                              SizedBox(height: 12),
                              Text(
                                'Scanning for devices on local network...',
                                style: TextStyle(
                                  color: NeobrutalTheme.ink,
                                  fontWeight: FontWeight.w700,
                                ),
                              ),
                            ],
                          ),
                        ),
                      ),
                    )
                  else
                    ListView.separated(
                      shrinkWrap: true,
                      physics: const NeverScrollableScrollPhysics(),
                      itemCount: _devices.length,
                      separatorBuilder: (context, index) =>
                          const SizedBox(height: 8),
                      itemBuilder: (context, index) {
                        final dev = _devices[index];
                        IconData iconData = Icons.devices;
                        if (dev.osType.toLowerCase().contains('windows')) {
                          iconData = Icons.desktop_windows;
                        } else if (dev.osType
                            .toLowerCase()
                            .contains('android')) {
                          iconData = Icons.phone_android;
                        }

                        return NeobrutalPressable(
                          cornerRadius: 4,
                          shadowOffset: const Offset(3, 3),
                          child: Container(
                            decoration: NeobrutalTheme.compactPanel(
                              color: NeobrutalTheme.surface,
                              shadow: false,
                            ),
                            child: Material(
                              color: Colors.transparent,
                              child: ListTile(
                                key: Key('device_tile_${dev.deviceId}'),
                                leading: Container(
                                  width: 40,
                                  height: 40,
                                  decoration: BoxDecoration(
                                    color: NeobrutalTheme.mint,
                                    border: Border.all(
                                      color: NeobrutalTheme.ink,
                                      width: NeobrutalTheme.compactBorderWidth,
                                    ),
                                    borderRadius: BorderRadius.circular(4),
                                  ),
                                  child: Icon(
                                    iconData,
                                    color: NeobrutalTheme.ink,
                                  ),
                                ),
                                title: Text(
                                  dev.deviceName,
                                  style: const TextStyle(
                                    fontWeight: FontWeight.w900,
                                    color: NeobrutalTheme.ink,
                                  ),
                                ),
                                subtitle: Row(
                                  mainAxisSize: MainAxisSize.min,
                                  children: [
                                    Flexible(
                                      child: Text(
                                        '${dev.ipAddress}:${dev.port} • ${dev.osType}',
                                        overflow: TextOverflow.ellipsis,
                                        style: const TextStyle(
                                          color: NeobrutalTheme.muted,
                                          fontWeight: FontWeight.w600,
                                        ),
                                      ),
                                    ),
                                    if (UnattendedStorage.hasSavedPassword(
                                          dev.deviceId,
                                        ) ||
                                        UnattendedStorage.hasSavedPassword(
                                          dev.ipAddress,
                                        )) ...[
                                      const SizedBox(width: 6),
                                      Container(
                                        padding: const EdgeInsets.symmetric(
                                          horizontal: 5,
                                          vertical: 2,
                                        ),
                                        decoration: BoxDecoration(
                                          color: NeobrutalTheme.yellow,
                                          border: Border.all(
                                            color: NeobrutalTheme.ink,
                                            width: 1.5,
                                          ),
                                          borderRadius: BorderRadius.circular(
                                            3,
                                          ),
                                        ),
                                        child: const Row(
                                          mainAxisSize: MainAxisSize.min,
                                          children: [
                                            Icon(
                                              Icons.vpn_key,
                                              size: 12,
                                              color: NeobrutalTheme.ink,
                                            ),
                                            SizedBox(width: 3),
                                            Text(
                                              'Saved',
                                              style: TextStyle(
                                                fontSize: 10,
                                                fontWeight: FontWeight.w900,
                                                color: NeobrutalTheme.ink,
                                              ),
                                            ),
                                          ],
                                        ),
                                      ),
                                    ],
                                  ],
                                ),
                                trailing: NeobrutalPressable(
                                  shadowOffset: const Offset(2, 2),
                                  child: ElevatedButton(
                                    key: Key('connect_button_${dev.deviceId}'),
                                    onPressed: () => _showPinDialog(dev),
                                    child: const Text('Connect'),
                                  ),
                                ),
                              ),
                            ),
                          ),
                        );
                      },
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
