import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import '../models/device.dart';
import '../widgets/pin_dialog.dart';
import 'mirror_view.dart';

class HomeView extends StatefulWidget {
  final String myDeviceId;
  final List<DiscoveredDevice> initialDevices;
  final void Function(DiscoveredDevice device, String pin)? onConnect;

  const HomeView({
    super.key,
    required this.myDeviceId,
    this.initialDevices = const [],
    this.onConnect,
  });

  @override
  State<HomeView> createState() => _HomeViewState();
}

class _HomeViewState extends State<HomeView> {
  late List<DiscoveredDevice> _devices;
  bool _isCopied = false;

  @override
  void initState() {
    super.initState();
    _devices = List.from(widget.initialDevices);
  }

  @override
  void didUpdateWidget(covariant HomeView oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.initialDevices != widget.initialDevices) {
      _devices = List.from(widget.initialDevices);
    }
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

  void _showDirectIpDialog() {
    final ipController = TextEditingController(text: '10.0.2.2');
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
              'Enter the Host PC IP address running VrV Desk:',
              style: TextStyle(fontSize: 13, color: Colors.grey),
            ),
            const SizedBox(height: 12),
            TextField(
              key: const Key('direct_ip_field'),
              controller: ipController,
              decoration: const InputDecoration(
                labelText: 'Host IP Address',
                border: OutlineInputBorder(),
                hintText: '10.0.2.2 or 192.168.x.x',
              ),
              keyboardType: TextInputType.url,
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
              final ip = ipController.text.trim();
              final pin = pinController.text.trim();
              if (ip.isNotEmpty) {
                Navigator.of(dialogContext).pop();
                Navigator.of(context).push(
                  MaterialPageRoute(
                    builder: (_) => MirrorView(
                      hostIp: ip,
                      port: 53211,
                      initialPin: pin.isNotEmpty ? pin : null,
                    ),
                  ),
                );
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
                    ElevatedButton.icon(
                      key: const Key('copy_device_id_button'),
                      onPressed: _copyDeviceId,
                      icon: Icon(_isCopied ? Icons.check : Icons.copy),
                      label: Text(_isCopied ? 'Copied' : 'Copy ID'),
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
