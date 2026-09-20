import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:vrv_desk/src/services/file_transfer_service.dart';
import 'package:vrv_desk/src/views/file_manager_view.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  group('FileTransferService Unit Tests', () {
    late StreamController<dynamic> incomingController;
    late List<Map<String, dynamic>> sentMessages;
    late FileTransferService service;

    setUp(() {
      incomingController = StreamController<dynamic>.broadcast();
      sentMessages = [];
      service = FileTransferService(
        incomingStream: incomingController.stream,
        sendMessage: (msg) {
          sentMessages.add(jsonDecode(msg) as Map<String, dynamic>);
        },
      );
    });

    tearDown(() {
      service.dispose();
      incomingController.close();
    });

    test('listRemoteDirectory sends fs_list and parses response', () async {
      final future = service.listRemoteDirectory('C:\\TestDir');

      expect(sentMessages.length, 1);
      final req = sentMessages.first;
      expect(req['type'], 'fs_list');
      expect(req['path'], 'C:\\TestDir');
      final reqId = req['id'] as String;

      incomingController.add(jsonEncode({
        'type': 'fs_list_resp',
        'id': reqId,
        'path': 'C:\\TestDir',
        'entries': [
          {'name': 'SubFolder', 'is_dir': true, 'size': 0, 'modified_ms': 1000},
          {'name': 'document.txt', 'is_dir': false, 'size': 2048, 'modified_ms': 2000},
        ],
      }));

      final res = await future;
      expect(res.path, 'C:\\TestDir');
      expect(res.entries.length, 2);
      expect(res.entries[0].name, 'SubFolder');
      expect(res.entries[0].isDir, true);
      expect(res.entries[1].name, 'document.txt');
      expect(res.entries[1].size, 2048);
      expect(res.entries[1].formattedSize, '2.0 KB');
    });

    test('downloadFile requests chunks and writes local file', () async {
      final tempDir = Directory.systemTemp.createTempSync('vrv_dl_test');
      final localTarget = '${tempDir.path}/downloaded.bin';

      final progressList = <double>[];
      final dlFuture = service.downloadFile(
        'C:\\Remote\\file.bin',
        localTarget,
        onProgress: (prog, rec, tot) => progressList.add(prog),
      );

      // Give event loop time to send first request
      await Future.delayed(const Duration(milliseconds: 10));
      expect(sentMessages.isNotEmpty, true);
      final req1 = sentMessages.first;
      expect(req1['type'], 'fs_read_chunk');
      expect(req1['offset'], 0);

      // Simulate first 10 bytes chunk with EOF
      final chunkBytes = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
      incomingController.add(jsonEncode({
        'type': 'fs_read_resp',
        'id': req1['id'],
        'path': 'C:\\Remote\\file.bin',
        'offset': 0,
        'total_size': 10,
        'eof': true,
        'data_b64': base64Encode(chunkBytes),
      }));

      await dlFuture;
      expect(File(localTarget).existsSync(), true);
      expect(File(localTarget).readAsBytesSync(), chunkBytes);
      expect(progressList.isNotEmpty, true);

      tempDir.deleteSync(recursive: true);
    });

    test('uploadFile reads local file and sends chunks', () async {
      final tempDir = Directory.systemTemp.createTempSync('vrv_ul_test');
      final localSource = '${tempDir.path}/upload.bin';
      final fileData = [65, 66, 67, 68, 69];
      File(localSource).writeAsBytesSync(fileData);

      final ulFuture = service.uploadFile(localSource, 'C:\\Remote\\upload.bin');

      // Wait for write chunk request
      await Future.delayed(const Duration(milliseconds: 10));
      expect(sentMessages.isNotEmpty, true);
      final req1 = sentMessages.first;
      expect(req1['type'], 'fs_write_chunk');
      expect(req1['offset'], 0);
      expect(req1['eof'], true);
      expect(req1['data_b64'], base64Encode(fileData));

      // Respond success
      incomingController.add(jsonEncode({
        'type': 'fs_write_resp',
        'id': req1['id'],
        'path': 'C:\\Remote\\upload.bin',
        'bytes_written': fileData.length,
        'eof': true,
        'success': true,
      }));

      await ulFuture;
      tempDir.deleteSync(recursive: true);
    });

    test('createRemoteDirectory and deleteRemoteItem send correct actions', () async {
      final mkdirFuture = service.createRemoteDirectory('C:\\NewFolder');
      await Future.delayed(const Duration(milliseconds: 10));
      final mkdirReq = sentMessages.last;
      expect(mkdirReq['type'], 'fs_mkdir');
      expect(mkdirReq['path'], 'C:\\NewFolder');

      incomingController.add(jsonEncode({
        'type': 'fs_action_resp',
        'id': mkdirReq['id'],
        'action': 'mkdir',
        'success': true,
      }));
      final mkdirRes = await mkdirFuture;
      expect(mkdirRes, true);

      final delFuture = service.deleteRemoteItem('C:\\NewFolder', true);
      await Future.delayed(const Duration(milliseconds: 10));
      final delReq = sentMessages.last;
      expect(delReq['type'], 'fs_delete');
      expect(delReq['is_dir'], true);

      incomingController.add(jsonEncode({
        'type': 'fs_action_resp',
        'id': delReq['id'],
        'action': 'delete',
        'success': true,
      }));
      final delRes = await delFuture;
      expect(delRes, true);
    });
  });

  group('FileManagerView Widget Tests', () {
    late StreamController<dynamic> incomingController;
    late FileTransferService service;

    setUp(() {
      incomingController = StreamController<dynamic>.broadcast();
      service = FileTransferService(
        incomingStream: incomingController.stream,
        sendMessage: (msg) {
          final json = jsonDecode(msg) as Map<String, dynamic>;
          if (json['type'] == 'fs_list') {
            incomingController.add(jsonEncode({
              'type': 'fs_list_resp',
              'id': json['id'],
              'path': 'C:\\MockRemote',
              'entries': [
                {'name': 'RemoteFolder', 'is_dir': true, 'size': 0, 'modified_ms': 1000},
                {'name': 'sample_data.csv', 'is_dir': false, 'size': 512000, 'modified_ms': 2000},
              ],
            }));
          }
        },
      );
    });

    tearDown(() {
      service.dispose();
      incomingController.close();
    });

    testWidgets('Renders File Manager View and displays remote files', (tester) async {
      await tester.pumpWidget(
        MaterialApp(
          home: FileManagerView(
            service: service,
            remoteHostName: 'DESKTOP-TEST',
          ),
        ),
      );

      await tester.pumpAndSettle();

      expect(find.text('File Manager • DESKTOP-TEST'), findsOneWidget);
      expect(find.text('Download to Local'), findsOneWidget);
      expect(find.text('Upload to Remote'), findsOneWidget);

      // Check remote tab or content
      if (find.text('Remote Files').evaluate().isNotEmpty) {
        await tester.tap(find.text('Remote Files'));
        await tester.pumpAndSettle();
      }

      expect(find.text('sample_data.csv'), findsOneWidget);
      expect(find.text('RemoteFolder'), findsOneWidget);

      // Tap on sample_data.csv to select it
      await tester.tap(find.text('sample_data.csv'));
      await tester.pumpAndSettle();

      expect(find.text('Remote: sample_data.csv'), findsOneWidget);
    });
  });
}
