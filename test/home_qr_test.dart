import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:vrv_desk/src/views/home_view.dart';
import 'package:vrv_desk/src/widgets/qr_code_dialog.dart';

void main() {
  testWidgets('HomeView tapping Show QR opens QrCodeDialog', (tester) async {
    await tester.pumpWidget(
      const MaterialApp(
        home: HomeView(myDeviceId: '684 174'),
      ),
    );

    expect(find.byKey(const Key('show_qr_button')), findsOneWidget);
    await tester.tap(find.byKey(const Key('show_qr_button')));
    await tester.pump(const Duration(milliseconds: 300));

    expect(find.byType(QrCodeDialog), findsOneWidget);
    expect(find.text('Device QR Code'), findsOneWidget);
    expect(find.byKey(const Key('qr_image_view')), findsOneWidget);
  });
}
