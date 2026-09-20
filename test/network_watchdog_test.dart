import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:vrv_desk/src/views/mirror_view.dart';

class MockWebSocketStream implements WebSocket {
  final StreamController<dynamic> _controller = StreamController<dynamic>.broadcast();
  final List<dynamic> sentMessages = [];
  bool isClosed = false;

  void emit(dynamic data) {
    if (!isClosed) {
      _controller.add(data);
    }
  }

  @override
  StreamSubscription listen(void Function(dynamic event)? onData,
      {Function? onError, void Function()? onDone, bool? cancelOnError}) {
    return _controller.stream.listen(onData, onError: onError, onDone: onDone, cancelOnError: cancelOnError);
  }

  @override
  void add(dynamic data) {
    sentMessages.add(data);
  }

  @override
  Future close([int? code, String? reason]) async {
    isClosed = true;
    await _controller.close();
  }

  @override
  dynamic noSuchMethod(Invocation invocation) => super.noSuchMethod(invocation);
}

void main() {
  group('Network Watchdog & Auto-Reconnect Overlay Tests', () {
    testWidgets('triggers reconnect overlay after 3.0s media buffer timeout and dismisses when resumed', (tester) async {
      final mockSocket = MockWebSocketStream();

      await tester.pumpWidget(
        MaterialApp(
          home: MirrorView(
            hostIp: '127.0.0.1',
            port: 53211,
            initialPin: '123456',
            webSocketConnector: (url) async => mockSocket,
          ),
        ),
      );
      await tester.pump();

      // Host requests auth -> client sends verify -> host replies auth_ok
      mockSocket.emit(jsonEncode({'type': 'auth_required'}));
      await tester.pump();

      mockSocket.emit(jsonEncode({'type': 'auth_ok', 'session_token': 'test_token_123', 'e2ee': false}));
      await tester.pump();

      // Send initial audio frame (VAUD)
      final vaudPacket = Uint8List.fromList([0x56, 0x41, 0x55, 0x44, 0x01, 0x02, 0x80, 0xBB, 0, 0, 0, 0]);
      mockSocket.emit(vaudPacket);
      await tester.pump();

      // Overlay should not be visible
      expect(find.byKey(const Key('network_reconnect_overlay')), findsNothing);

      // Fast-forward 2 seconds: still within 3.0s buffer timeout
      await tester.pump(const Duration(seconds: 2));
      expect(find.byKey(const Key('network_reconnect_overlay')), findsNothing);

      // Fast-forward past 3.0s: watchdog should trigger reconnect overlay
      await tester.pump(const Duration(seconds: 2));
      expect(find.byKey(const Key('network_reconnect_overlay')), findsOneWidget);
      expect(find.text('Mencoba menghubungkan kembali...'), findsOneWidget);

      // Packets resume: send frame
      mockSocket.emit(vaudPacket);
      await tester.pump();

      // Reconnect overlay automatically dismisses
      expect(find.byKey(const Key('network_reconnect_overlay')), findsNothing);
    });
  });
}
