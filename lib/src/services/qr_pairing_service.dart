import 'dart:convert';

class QrPairingData {
  final String deviceId;
  final String? ipAddress;
  final int port;
  final String? pin;
  final String? signalingUrl;

  const QrPairingData({
    required this.deviceId,
    this.ipAddress,
    this.port = 53211,
    this.pin,
    this.signalingUrl,
  });

  Map<String, dynamic> toJson() => {
        'type': 'vrv_pairing',
        'id': deviceId,
        if (ipAddress != null) 'ip': ipAddress,
        'port': port,
        if (pin != null) 'pin': pin,
        if (signalingUrl != null) 'sig': signalingUrl,
      };
}

class QrPairingService {
  static const String uriPrefix = 'vrvdesk://connect';

  static String serialize(QrPairingData data) {
    return jsonEncode(data.toJson());
  }

  static String serializeAsUri(QrPairingData data) {
    final queryParams = <String, String>{
      'id': data.deviceId,
      if (data.ipAddress != null) 'ip': data.ipAddress!,
      'port': data.port.toString(),
      if (data.pin != null) 'pin': data.pin!,
      if (data.signalingUrl != null) 'sig': data.signalingUrl!,
    };
    return Uri(
      scheme: 'vrvdesk',
      host: 'connect',
      queryParameters: queryParams,
    ).toString();
  }

  static QrPairingData? deserialize(String raw) {
    final trimmed = raw.trim();
    if (trimmed.isEmpty) return null;

    // 1. Try JSON
    if (trimmed.startsWith('{') && trimmed.endsWith('}')) {
      try {
        final decoded = jsonDecode(trimmed);
        if (decoded is Map<String, dynamic>) {
          if (decoded['type'] == 'vrv_pairing' ||
              decoded.containsKey('id') ||
              decoded.containsKey('ip')) {
            final id = (decoded['id'] ?? decoded['device_id'] ?? '').toString();
            final ip = decoded['ip']?.toString();
            final port = (decoded['port'] is int)
                ? decoded['port'] as int
                : int.tryParse(decoded['port']?.toString() ?? '') ?? 53211;
            final pin = decoded['pin']?.toString();
            final sig = decoded['sig']?.toString();

            if (id.isNotEmpty || (ip != null && ip.isNotEmpty)) {
              return QrPairingData(
                deviceId: id,
                ipAddress: ip,
                port: port,
                pin: pin,
                signalingUrl: sig,
              );
            }
          }
        }
      } catch (_) {}
    }

    // 2. Try URI (vrvdesk://connect?...)
    if (trimmed.startsWith('vrvdesk://')) {
      try {
        final uri = Uri.parse(trimmed);
        final q = uri.queryParameters;
        final id = q['id'] ?? '';
        final ip = q['ip'];
        final port = int.tryParse(q['port'] ?? '') ?? 53211;
        final pin = q['pin'];
        final sig = q['sig'];

        if (id.isNotEmpty || (ip != null && ip.isNotEmpty)) {
          return QrPairingData(
            deviceId: id,
            ipAddress: ip,
            port: port,
            pin: pin,
            signalingUrl: sig,
          );
        }
      } catch (_) {}
    }

    // 3. Try plain text: "id:pin" (e.g. "684174:829104") or "ip:port:pin"
    if (trimmed.contains(':')) {
      final parts = trimmed.split(':');
      if (parts.length == 2) {
        final part0 = parts[0].trim();
        final part1 = parts[1].trim();
        if (part0.length == 6 && RegExp(r'^\d+$').hasMatch(part0)) {
          return QrPairingData(
            deviceId: part0,
            pin: part1,
          );
        }
      } else if (parts.length == 3) {
        final ip = parts[0].trim();
        final port = int.tryParse(parts[1].trim()) ?? 53211;
        final pin = parts[2].trim();
        return QrPairingData(
          deviceId: '',
          ipAddress: ip,
          port: port,
          pin: pin,
        );
      }
    }

    return null;
  }
}
