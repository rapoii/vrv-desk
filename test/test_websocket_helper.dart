import 'dart:async';
import 'dart:convert';
import 'dart:io';

class MockWebSocket extends Stream<dynamic> implements WebSocket {
  final StreamController<dynamic> _incomingController = StreamController<dynamic>.broadcast();
  final StreamController<dynamic> _outgoingController = StreamController<dynamic>.broadcast();
  bool _isClosed = false;

  void feedIncoming(dynamic data) {
    if (!_isClosed) {
      _incomingController.add(data);
    }
  }

  Stream<dynamic> get outgoing => _outgoingController.stream;

  @override
  StreamSubscription<dynamic> listen(
    void Function(dynamic event)? onData, {
    Function? onError,
    void Function()? onDone,
    bool? cancelOnError,
  }) {
    return _incomingController.stream.listen(
      onData,
      onError: onError,
      onDone: onDone,
      cancelOnError: cancelOnError,
    );
  }

  @override
  void add(dynamic data) {
    if (!_isClosed) {
      _outgoingController.add(data);
    }
  }

  @override
  void addUtf8Text(List<int> bytes) {
    if (!_isClosed) {
      _outgoingController.add(utf8.decode(bytes));
    }
  }

  @override
  Future close([int? code, String? reason]) async {
    _isClosed = true;
    await _incomingController.close();
    await _outgoingController.close();
  }

  @override
  Duration? pingInterval;

  @override
  int? get closeCode => _isClosed ? 1000 : null;

  @override
  String? get closeReason => _isClosed ? 'normal closure' : null;

  @override
  String get extensions => '';

  @override
  String get protocol => 'chat';

  @override
  int get readyState => _isClosed ? WebSocket.closed : WebSocket.open;

  @override
  Future get done => _incomingController.done;

  @override
  void addError(Object error, [StackTrace? stackTrace]) {
    _incomingController.addError(error, stackTrace);
  }

  @override
  Future addStream(Stream stream) => _incomingController.addStream(stream);
}
