import 'dart:async';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:vrv_desk/src/services/android_host_service.dart';
import 'package:vrv_desk/src/widgets/host_mode_dialog.dart';
import 'package:vrv_desk/src/views/home_view.dart';
import 'package:qr_flutter/qr_flutter.dart';

class FakeAndroidHostService extends AndroidHostService {
  bool _mockRunning = false;
  String _mockPin = '';
  int _mockFrames = 0;
  int _mockAuthClients = 0;
  bool _mockAccessibility = true;
  bool _mockAudioSupported = true;
  bool openSettingsCalled = false;
  bool startCaptureCalled = false;
  bool stopCaptureCalled = false;
  bool startCalled = false;
  bool stopCalled = false;

  FakeAndroidHostService({
    bool initialRunning = false,
    String initialPin = '654321',
    bool initialAccessibility = true,
    bool initialAudioSupported = true,
  })  : _mockRunning = initialRunning,
        _mockPin = initialPin,
        _mockAccessibility = initialAccessibility,
        _mockAudioSupported = initialAudioSupported;

  @override
  bool get isRunning => _mockRunning;

  @override
  String get currentPin => _mockPin;

  @override
  int get framesSent => _mockFrames;

  @override
  int get authenticatedClientsCount => _mockAuthClients;

  @override
  Future<bool> isAccessibilityEnabled() async => _mockAccessibility;

  @override
  Future<bool> isInternalAudioSupported() async => _mockAudioSupported;

  @override
  Future<bool> openAccessibilitySettings() async {
    openSettingsCalled = true;
    return true;
  }

  @override
  Future<bool> startCapture({
    int resultCode = -1,
    dynamic intentData,
    int width = 1280,
    int height = 720,
    int bitrate = 2500000,
    int fps = 30,
    bool enableAudio = true,
  }) async {
    startCaptureCalled = true;
    return true;
  }

  @override
  Future<bool> stopCapture() async {
    stopCaptureCalled = true;
    return true;
  }

  @override
  Future<void> start({
    dynamic bindAddress,
    int port = AndroidHostService.defaultPort,
    String? pin,
    String? deviceId,
    String? deviceName,
    bool enableUdpBeacon = true,
  }) async {
    startCalled = true;
    _mockRunning = true;
    _mockPin = pin ?? '654321';
  }

  @override
  Future<void> stop() async {
    stopCalled = true;
    _mockRunning = false;
    _mockPin = '';
  }

  void updateMetrics({int frames = 0, int clients = 0}) {
    _mockFrames = frames;
    _mockAuthClients = clients;
  }
}

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  group('HostModeDialog Widget Tests', () {
    testWidgets('Renders idle broadcast status and start button', (tester) async {
      final fakeService = FakeAndroidHostService(initialRunning: false);

      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: HostModeDialog(
              hostService: fakeService,
              deviceId: '123456',
              enableMetricsTimer: false,
            ),
          ),
        ),
      );

      expect(find.text('Broadcast Screen (Host)'), findsOneWidget);
      expect(find.byKey(const Key('host_mode_toggle_button')), findsOneWidget);
      expect(find.text('Start Broadcasting'), findsOneWidget);
      expect(find.text('Broadcasting Inactive'), findsOneWidget);
    });

    testWidgets('Shows accessibility warning and settings button when disabled', (tester) async {
      final fakeService = FakeAndroidHostService(
        initialRunning: false,
        initialAccessibility: false,
      );

      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: HostModeDialog(
              hostService: fakeService,
              deviceId: '123456',
              enableMetricsTimer: false,
            ),
          ),
        ),
      );
      await tester.pump(const Duration(milliseconds: 100));

      expect(find.byKey(const Key('accessibility_warning_card')), findsOneWidget);
      expect(find.text('Accessibility Permission Needed'), findsOneWidget);

      final openBtn = find.byKey(const Key('open_accessibility_settings_button'));
      expect(openBtn, findsOneWidget);

      await tester.tap(openBtn);
      await tester.pump();
      expect(fakeService.openSettingsCalled, isTrue);
    });

    testWidgets('Toggles broadcasting on: starts service, displays PIN and QR code', (tester) async {
      final fakeService = FakeAndroidHostService(initialRunning: false);

      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: HostModeDialog(
              hostService: fakeService,
              deviceId: '123456',
              enableMetricsTimer: false,
            ),
          ),
        ),
      );

      final toggleBtn = find.byKey(const Key('host_mode_toggle_button'));
      await tester.tap(toggleBtn);
      await tester.pump(const Duration(milliseconds: 100));

      expect(fakeService.startCalled, isTrue);
      expect(fakeService.startCaptureCalled, isTrue);
      expect(find.text('Stop Broadcasting'), findsOneWidget);
      expect(find.byKey(const Key('host_pin_display')), findsOneWidget);
      expect(find.textContaining('654 321'), findsOneWidget);
      expect(find.byType(QrImageView), findsOneWidget);
    });

    testWidgets('Displays frame counter and connected client status when running', (tester) async {
      final fakeService = FakeAndroidHostService(
        initialRunning: true,
        initialPin: '998877',
      );
      fakeService.updateMetrics(frames: 142, clients: 1);

      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: HostModeDialog(
              hostService: fakeService,
              deviceId: '987654',
              enableMetricsTimer: false,
            ),
          ),
        ),
      );
      await tester.pump(const Duration(milliseconds: 100));

      expect(find.text('Stop Broadcasting'), findsOneWidget);
      expect(find.text('142'), findsOneWidget); // frames sent
      expect(find.textContaining('1 connected'), findsOneWidget); // connected client
    });

    testWidgets('Toggles broadcasting off: stops service and capture', (tester) async {
      final fakeService = FakeAndroidHostService(
        initialRunning: true,
        initialPin: '998877',
      );

      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: HostModeDialog(
              hostService: fakeService,
              deviceId: '987654',
              enableMetricsTimer: false,
            ),
          ),
        ),
      );

      final toggleBtn = find.byKey(const Key('host_mode_toggle_button'));
      await tester.ensureVisible(toggleBtn);
      await tester.tap(toggleBtn);
      await tester.pump(const Duration(milliseconds: 100));

      expect(fakeService.stopCalled, isTrue);
      expect(fakeService.stopCaptureCalled, isTrue);
      expect(find.text('Start Broadcasting'), findsOneWidget);
    });

    testWidgets('renders audio fallback warning when OS is below Android 10', (tester) async {
      final fakeService = FakeAndroidHostService(
        initialRunning: false,
        initialAudioSupported: false,
      );

      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: HostModeDialog(
              hostService: fakeService,
              deviceId: '123456',
              enableMetricsTimer: false,
            ),
          ),
        ),
      );
      await tester.pump();

      expect(find.byKey(const Key('audio_fallback_warning_card')), findsOneWidget);
      expect(find.textContaining('Internal audio capture requires Android 10+'), findsOneWidget);
    });

    testWidgets('renders audio active badge when internal audio is supported', (tester) async {
      final fakeService = FakeAndroidHostService(
        initialRunning: false,
        initialAudioSupported: true,
      );

      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: HostModeDialog(
              hostService: fakeService,
              deviceId: '123456',
              enableMetricsTimer: false,
            ),
          ),
        ),
      );
      await tester.pump();

      expect(find.byKey(const Key('audio_active_badge')), findsOneWidget);
      expect(find.textContaining('Internal Audio (48kHz Stereo) Supported'), findsOneWidget);
    });
  });

  group('HomeView Share Screen Integration Tests', () {
    testWidgets('HomeView has Share My Screen button and opens HostModeDialog', (tester) async {
      final fakeService = FakeAndroidHostService();

      await tester.pumpWidget(
        MaterialApp(
          home: HomeView(
            myDeviceId: '123456',
            isAndroidOverride: true,
            androidHostService: fakeService,
          ),
        ),
      );

      final shareBtn = find.byKey(const Key('share_screen_button'));
      expect(shareBtn, findsOneWidget);

      await tester.tap(shareBtn);
      await tester.pump(const Duration(milliseconds: 300));

      expect(find.byType(HostModeDialog), findsOneWidget);
      expect(find.text('Broadcast Screen (Host)'), findsOneWidget);
    });
  });
}
