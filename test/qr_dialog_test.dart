import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:vrv_desk/src/services/qr_pairing_service.dart';
import 'package:vrv_desk/src/widgets/qr_code_dialog.dart';

void main() {
  testWidgets('QrCodeDialog displays device ID and QR code', (tester) async {
    const data = QrPairingData(
      deviceId: '684174',
      ipAddress: '192.168.1.5',
      port: 53211,
      pin: '829104',
    );

    await tester.pumpWidget(
      const MaterialApp(
        home: Scaffold(
          body: QrCodeDialog(pairingData: data),
        ),
      ),
    );

    expect(find.text('Device QR Code'), findsOneWidget);
    expect(find.text('Device ID: 684 174'), findsOneWidget);
    expect(find.text('PIN: 829 104'), findsOneWidget);
    expect(find.byKey(const Key('qr_image_view')), findsOneWidget);
    expect(find.text('Close'), findsOneWidget);
  });
}
