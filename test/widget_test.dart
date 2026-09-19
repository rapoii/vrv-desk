import 'package:flutter_test/flutter_test.dart';
import 'package:vrv_desk/main.dart';

void main() {
  testWidgets('VrvDeskApp boots into HomeView with Device ID', (WidgetTester tester) async {
    await tester.pumpWidget(const VrvDeskApp());
    await tester.pump();

    // Verify HomeView elements
    expect(find.text('Mirror & Remote Control'), findsOneWidget);
    expect(find.text('Your Device ID'), findsOneWidget);
    expect(find.text('Discovered Devices'), findsOneWidget);
  });
}
