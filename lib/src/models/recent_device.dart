class RecentDevice {
  final String deviceId;
  final String name;
  final String? ip;
  final String osType; // 'pc' or 'android'
  final DateTime lastConnected;
  final String? savedPin;

  const RecentDevice({
    required this.deviceId,
    required this.name,
    this.ip,
    this.osType = 'pc',
    required this.lastConnected,
    this.savedPin,
  });

  Map<String, dynamic> toJson() {
    return {
      'deviceId': deviceId,
      'name': name,
      'ip': ip,
      'osType': osType,
      'lastConnected': lastConnected.toIso8601String(),
      'savedPin': savedPin,
    };
  }

  factory RecentDevice.fromJson(Map<String, dynamic> json) {
    return RecentDevice(
      deviceId: json['deviceId'] as String? ?? '',
      name: json['name'] as String? ?? 'Remote Device',
      ip: json['ip'] as String?,
      osType: json['osType'] as String? ?? 'pc',
      lastConnected: json['lastConnected'] != null
          ? DateTime.tryParse(json['lastConnected'] as String) ?? DateTime.now()
          : DateTime.now(),
      savedPin: json['savedPin'] as String?,
    );
  }

  RecentDevice copyWith({
    String? deviceId,
    String? name,
    String? ip,
    String? osType,
    DateTime? lastConnected,
    String? savedPin,
  }) {
    return RecentDevice(
      deviceId: deviceId ?? this.deviceId,
      name: name ?? this.name,
      ip: ip ?? this.ip,
      osType: osType ?? this.osType,
      lastConnected: lastConnected ?? this.lastConnected,
      savedPin: savedPin ?? this.savedPin,
    );
  }
}
