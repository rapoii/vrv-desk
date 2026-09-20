import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:qr_flutter/qr_flutter.dart';

import '../services/qr_pairing_service.dart';
import '../theme/neobrutalist_theme.dart';

class QrCodeDialog extends StatelessWidget {
  final QrPairingData pairingData;

  const QrCodeDialog({super.key, required this.pairingData});

  String _format6Digit(String id) {
    final clean = id.replaceAll(' ', '');
    if (clean.length == 6) {
      return '${clean.substring(0, 3)} ${clean.substring(3)}';
    }
    return id;
  }

  @override
  Widget build(BuildContext context) {
    final payload = QrPairingService.serialize(pairingData);

    return AlertDialog(
      backgroundColor: NeobrutalTheme.paper,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(NeobrutalTheme.radius),
        side: const BorderSide(
          color: NeobrutalTheme.ink,
          width: NeobrutalTheme.borderWidth,
        ),
      ),
      title: const Row(
        children: [
          Icon(Icons.qr_code_2, color: NeobrutalTheme.ink),
          SizedBox(width: 8),
          Text(
            'Device QR Code',
            style: TextStyle(
              color: NeobrutalTheme.ink,
              fontSize: 18,
              fontWeight: FontWeight.w900,
            ),
          ),
        ],
      ),
      content: SingleChildScrollView(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            const Text(
              'Scan this code from another device to connect instantly without typing.',
              style: TextStyle(
                color: NeobrutalTheme.ink,
                fontSize: 13,
                fontWeight: FontWeight.w600,
              ),
              textAlign: TextAlign.center,
            ),
            const SizedBox(height: 16),
            SizedBox(
              width: 224,
              height: 224,
              child: Container(
                padding: const EdgeInsets.all(12),
                decoration: NeobrutalTheme.panel(color: NeobrutalTheme.surface),
                child: QrImageView(
                  key: const Key('qr_image_view'),
                  data: payload,
                  version: QrVersions.auto,
                  size: 200.0,
                ),
              ),
            ),
            const SizedBox(height: 16),
            if (pairingData.deviceId.isNotEmpty)
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
                  'Device ID: ${_format6Digit(pairingData.deviceId)}',
                  style: const TextStyle(
                    color: NeobrutalTheme.ink,
                    fontWeight: FontWeight.w900,
                    letterSpacing: 1.1,
                  ),
                ),
              ),
            if (pairingData.pin != null && pairingData.pin!.isNotEmpty) ...[
              const SizedBox(height: 8),
              Container(
                padding: const EdgeInsets.symmetric(
                  horizontal: 12,
                  vertical: 6,
                ),
                decoration: NeobrutalTheme.compactPanel(
                  color: NeobrutalTheme.cyan,
                  shadow: false,
                ),
                child: Text(
                  'PIN: ${_format6Digit(pairingData.pin!)}',
                  style: const TextStyle(
                    color: NeobrutalTheme.ink,
                    fontWeight: FontWeight.w900,
                    letterSpacing: 1.1,
                  ),
                ),
              ),
            ],
          ],
        ),
      ),
      actions: [
        TextButton.icon(
          icon: const Icon(Icons.copy, size: 16, color: NeobrutalTheme.ink),
          label: const Text(
            'Copy Data',
            style: TextStyle(
              color: NeobrutalTheme.ink,
              fontWeight: FontWeight.w800,
            ),
          ),
          onPressed: () {
            Clipboard.setData(ClipboardData(text: payload));
            ScaffoldMessenger.of(context).showSnackBar(
              const SnackBar(
                content: Text('Pairing data copied to clipboard'),
                duration: Duration(seconds: 2),
              ),
            );
          },
        ),
        ElevatedButton(
          style: ElevatedButton.styleFrom(
            backgroundColor: NeobrutalTheme.yellow,
            foregroundColor: NeobrutalTheme.ink,
          ),
          onPressed: () => Navigator.of(context).pop(),
          child: const Text('Close'),
        ),
      ],
    );
  }
}
