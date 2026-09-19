class DiscoveredDevice {
  final String deviceId;
  final String deviceName;
  final String osType;
  final String ipAddress;
  final int port;
  final DateTime lastSeen;

  DiscoveredDevice({
    required this.deviceId,
    required this.deviceName,
    required this.osType,
    required this.ipAddress,
    required this.port,
    required this.lastSeen,
  });

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is DiscoveredDevice &&
          runtimeType == other.runtimeType &&
          deviceId == other.deviceId &&
          deviceName == other.deviceName &&
          osType == other.osType &&
          ipAddress == other.ipAddress &&
          port == other.port;

  @override
  int get hashCode =>
      deviceId.hashCode ^
      deviceName.hashCode ^
      osType.hashCode ^
      ipAddress.hashCode ^
      port.hashCode;
}
