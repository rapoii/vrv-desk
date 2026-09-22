import 'package:flutter_test/flutter_test.dart';
import 'package:vrv_desk/src/models/recent_device.dart';
import 'package:vrv_desk/src/services/recent_devices_storage.dart';

void main() {
  group('RecentDevicesStorage Tests', () {
    setUp(() {
      RecentDevicesStorage.clear();
    });

    test('starts with empty recent devices list', () {
      expect(RecentDevicesStorage.getRecentDevices(), isEmpty);
    });

    test('addOrUpdate inserts new device at front', () {
      final now = DateTime.now();
      final dev1 = RecentDevice(
        deviceId: '123456',
        name: 'Device A',
        osType: 'pc',
        lastConnected: now,
      );

      RecentDevicesStorage.addOrUpdate(dev1);
      final list = RecentDevicesStorage.getRecentDevices();
      expect(list.length, 1);
      expect(list.first.deviceId, '123456');
      expect(list.first.name, 'Device A');
    });

    test('deduplicates devices by deviceId and moves to top', () {
      final now = DateTime.now();
      final dev1 = RecentDevice(
        deviceId: '123456',
        name: 'Device A',
        osType: 'pc',
        lastConnected: now,
      );
      final dev2 = RecentDevice(
        deviceId: '654321',
        name: 'Device B',
        osType: 'android',
        lastConnected: now,
      );
      final dev1Updated = RecentDevice(
        deviceId: '123 456', // formatted
        name: 'Device A Renamed',
        osType: 'pc',
        lastConnected: now.add(const Duration(minutes: 5)),
      );

      RecentDevicesStorage.addOrUpdate(dev1);
      RecentDevicesStorage.addOrUpdate(dev2);
      expect(RecentDevicesStorage.getRecentDevices().first.deviceId, '654321');

      RecentDevicesStorage.addOrUpdate(dev1Updated);
      final list = RecentDevicesStorage.getRecentDevices();
      expect(list.length, 2);
      expect(list.first.deviceId, '123 456');
      expect(list.first.name, 'Device A Renamed');
    });

    test('caps at maxRecent limit', () {
      for (int i = 0; i < 15; i++) {
        RecentDevicesStorage.addOrUpdate(
          RecentDevice(
            deviceId: 'dev_$i',
            name: 'Device $i',
            lastConnected: DateTime.now(),
          ),
        );
      }
      expect(RecentDevicesStorage.getRecentDevices().length, RecentDevicesStorage.maxRecent);
    });

    test('remove and clear work correctly', () {
      RecentDevicesStorage.addOrUpdate(
        RecentDevice(deviceId: 'dev_1', name: 'D1', lastConnected: DateTime.now()),
      );
      RecentDevicesStorage.addOrUpdate(
        RecentDevice(deviceId: 'dev_2', name: 'D2', lastConnected: DateTime.now()),
      );
      expect(RecentDevicesStorage.getRecentDevices().length, 2);

      RecentDevicesStorage.remove('dev_1');
      expect(RecentDevicesStorage.getRecentDevices().length, 1);
      expect(RecentDevicesStorage.getRecentDevices().first.deviceId, 'dev_2');

      RecentDevicesStorage.clear();
      expect(RecentDevicesStorage.getRecentDevices(), isEmpty);
    });
  });
}
