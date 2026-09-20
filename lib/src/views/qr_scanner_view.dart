import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:mobile_scanner/mobile_scanner.dart';

import '../services/qr_pairing_service.dart';
import '../theme/neobrutalist_theme.dart';

class QrScannerView extends StatefulWidget {
  final void Function(QrPairingData data)? onScanned;

  const QrScannerView({super.key, this.onScanned});

  @override
  State<QrScannerView> createState() => _QrScannerViewState();
}

class _QrScannerViewState extends State<QrScannerView> {
  final MobileScannerController _controller = MobileScannerController(
    detectionSpeed: DetectionSpeed.normal,
    facing: CameraFacing.back,
  );

  bool _isScanned = false;
  bool _isTorchOn = false;

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  void _handleBarcode(BarcodeCapture capture) {
    if (_isScanned) return;

    for (final barcode in capture.barcodes) {
      final rawValue = barcode.rawValue;
      if (rawValue != null && rawValue.isNotEmpty) {
        final pairingData = QrPairingService.deserialize(rawValue);
        if (pairingData != null) {
          setState(() {
            _isScanned = true;
          });
          HapticFeedback.mediumImpact();
          if (widget.onScanned != null) {
            widget.onScanned!(pairingData);
          } else {
            Navigator.of(context).pop(pairingData);
          }
          break;
        }
      }
    }
  }

  void _toggleTorch() {
    _controller.toggleTorch();
    setState(() {
      _isTorchOn = !_isTorchOn;
    });
  }

  void _switchCamera() {
    _controller.switchCamera();
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      backgroundColor: Colors.black,
      appBar: AppBar(
        backgroundColor: NeobrutalTheme.yellow,
        foregroundColor: NeobrutalTheme.ink,
        elevation: 0,
        shape: const Border(
          bottom: BorderSide(
            color: NeobrutalTheme.ink,
            width: NeobrutalTheme.borderWidth,
          ),
        ),
        title: const Text(
          'Scan QR Code',
          style: TextStyle(
            fontWeight: FontWeight.w900,
            fontSize: 18,
            color: NeobrutalTheme.ink,
          ),
        ),
        actions: [
          IconButton(
            key: const Key('toggle_torch_button'),
            icon: Icon(
              _isTorchOn ? Icons.flash_on : Icons.flash_off,
              color: NeobrutalTheme.ink,
            ),
            onPressed: _toggleTorch,
          ),
          IconButton(
            key: const Key('switch_camera_button'),
            icon: const Icon(
              Icons.flip_camera_android,
              color: NeobrutalTheme.ink,
            ),
            onPressed: _switchCamera,
          ),
        ],
      ),
      body: Stack(
        children: [
          MobileScanner(
            key: const Key('mobile_scanner_widget'),
            controller: _controller,
            onDetect: _handleBarcode,
          ),
          // Viewfinder reticle overlay
          Center(
            child: Container(
              width: 250,
              height: 250,
              decoration: BoxDecoration(
                border: Border.all(
                  color: NeobrutalTheme.yellow,
                  width: NeobrutalTheme.borderWidth,
                ),
                borderRadius: BorderRadius.circular(NeobrutalTheme.radius),
              ),
              child: Stack(
                children: [
                  Positioned(
                    top: 8,
                    left: 8,
                    child: Container(
                      width: 24,
                      height: 24,
                      decoration: const BoxDecoration(
                        border: Border(
                          top: BorderSide(color: Colors.white, width: 3),
                          left: BorderSide(color: Colors.white, width: 3),
                        ),
                      ),
                    ),
                  ),
                  Positioned(
                    top: 8,
                    right: 8,
                    child: Container(
                      width: 24,
                      height: 24,
                      decoration: const BoxDecoration(
                        border: Border(
                          top: BorderSide(color: Colors.white, width: 3),
                          right: BorderSide(color: Colors.white, width: 3),
                        ),
                      ),
                    ),
                  ),
                  Positioned(
                    bottom: 8,
                    left: 8,
                    child: Container(
                      width: 24,
                      height: 24,
                      decoration: const BoxDecoration(
                        border: Border(
                          bottom: BorderSide(color: Colors.white, width: 3),
                          left: BorderSide(color: Colors.white, width: 3),
                        ),
                      ),
                    ),
                  ),
                  Positioned(
                    bottom: 8,
                    right: 8,
                    child: Container(
                      width: 24,
                      height: 24,
                      decoration: const BoxDecoration(
                        border: Border(
                          bottom: BorderSide(color: Colors.white, width: 3),
                          right: BorderSide(color: Colors.white, width: 3),
                        ),
                      ),
                    ),
                  ),
                ],
              ),
            ),
          ),
          // Instruction label at bottom
          Positioned(
            bottom: 40,
            left: 20,
            right: 20,
            child: Column(
              children: [
                Container(
                  padding: const EdgeInsets.symmetric(
                    horizontal: 16,
                    vertical: 10,
                  ),
                  decoration: NeobrutalTheme.panel(color: NeobrutalTheme.paper),
                  child: const Text(
                    'Point camera at the Host Device QR code',
                    style: TextStyle(
                      color: NeobrutalTheme.ink,
                      fontSize: 13,
                      fontWeight: FontWeight.w800,
                    ),
                    textAlign: TextAlign.center,
                  ),
                ),
                const SizedBox(height: 16),
                TextButton(
                  onPressed: () => Navigator.of(context).pop(),
                  child: const Text(
                    'Enter PIN / IP Manually',
                    style: TextStyle(
                      color: Colors.white,
                      fontWeight: FontWeight.w800,
                    ),
                  ),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}
