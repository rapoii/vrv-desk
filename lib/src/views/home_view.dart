import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import '../models/device.dart';
import '../widgets/pin_dialog.dart';

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
          widget.onConnect?.call(device, pin);
        },
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('Mirror & Remote Control'),
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
