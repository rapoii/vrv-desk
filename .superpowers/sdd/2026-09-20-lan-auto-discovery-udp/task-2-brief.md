# Task 2 Brief: Flutter LAN Discovery Service & HomeView Live Device Integration

## Context & Objectives
You are implementing Task 2 of Phase 7 (Zero-Config LAN Discovery Engine via UDP) in `projects/vrv-desk/lib`.
This enables the Flutter Android client to listen for UDP discovery beacons from Windows PCs on port 53210 (multicast 239.255.42.99 and subnet broadcast), automatically populate the `HomeView` device list with discovered PCs, and connect with 1 tap.

## Requirements

1. **Create `lib/src/services/lan_discovery_service.dart`**:
   - Class `LanDiscoveryService`:
     - Constants:
       - `multicastAddress = '239.255.42.99'`
       - `discoveryPort = 53210`
       - `staleTimeout = Duration(seconds: 6)`
       - `pruneInterval = Duration(seconds: 2)`
     - State:
       - `RawDatagramSocket? _socket`
       - `final Map<String, DiscoveredDevice> _devices = {}`
       - `final StreamController<List<DiscoveredDevice>> _controller = StreamController.broadcast()`
       - `Timer? _pruneTimer`
     - Methods:
       - `Future<void> start()`:
         - Binds `RawDatagramSocket.bind(InternetAddress.anyIPv4, discoveryPort, reuseAddress: true, reusePort: true)`.
         - Safely calls `_socket?.joinMulticast(InternetAddress(multicastAddress))`.
         - Listens to socket events (`RawSocketEvent.read`), extracts datagram via `_socket?.receive()`.
         - Parses JSON datagram payload using `utf8.decode(datagram.data)`.
         - Resolves host IP: if `datagram.address.address` is loopback or any, or valid IP, uses `datagram.address.address`.
         - Creates/updates `DiscoveredDevice`:
           ```dart
           _devices[deviceId] = DiscoveredDevice(
             deviceId: deviceId,
             deviceName: deviceName,
             osType: osType,
             ipAddress: senderIp,
             port: port,
             lastSeen: DateTime.now(),
           );
           _controller.add(_devices.values.toList());
           ```
         - Starts `_pruneTimer`: periodically checks and removes devices where `DateTime.now().difference(device.lastSeen) > staleTimeout`. Emits updated list if any removed.
         - Gracefully catches socket errors or binding exceptions without crashing.
       - `Future<void> stop()`:
         - Cancels timer, closes socket, closes controller.
       - `Stream<List<DiscoveredDevice>> get devicesStream => _controller.stream;`
       - `List<DiscoveredDevice> get currentDevices => _devices.values.toList();`
       - Provide testable helper `void handleBeaconData(Uint8List data, String senderIp)` for unit testing without physical sockets.

2. **Integrate into `lib/src/views/home_view.dart`**:
   - In `_HomeViewState`:
     - Instantiate `late final LanDiscoveryService _lanService;`
     - `StreamSubscription<List<DiscoveredDevice>>? _lanSubscription;`
     - In `initState()`:
       - `_lanService = LanDiscoveryService();`
       - `_lanService.start();`
       - `_lanSubscription = _lanService.devicesStream.listen((devices) { setState(() { _devices = devices; }); });`
     - In `dispose()`:
       - `_lanSubscription?.cancel();`
       - `_lanService.stop();`
   - Ensure the `_devices` list displays the discovered cards with OS icon, device name, IP address, and a "Connect" button that opens `_showPinDialog(device)`.

3. **Write Unit & Widget Tests in `test/lan_discovery_test.dart`**:
   - Test 1: Beacon parsing and `DiscoveredDevice` generation via `handleBeaconData`.
   - Test 2: Pruning stale devices after timeout.
   - Test 3: Widget test verifying that when `devicesStream` emits a new device, `HomeView` updates its UI and displays the device name and IP.

4. **Verification**:
   - Flutter toolchain: `/d/Software/flutter/flutter/bin/flutter`
   - Run `flutter test test/lan_discovery_test.dart`
   - Run `flutter test` (all tests passing)
   - Run `flutter analyze` (0 issues found)

## Output Contract
Write full report to:
`projects/vrv-desk/.superpowers/sdd/2026-09-20-lan-auto-discovery-udp/task-2-report.md`
