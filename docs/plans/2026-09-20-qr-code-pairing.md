# Implementation Plan: Instant QR Code Pairing (Mobile & Desktop)

**Goal:** Implement zero-friction QR code pairing specified in `docs/ARCHITECTURE.md` (Section 4.3 & 5.2):
- Host/Device displays dynamic QR code with ID, IP/Signaling, and PIN.
- Client scans QR code with camera (`mobile_scanner`), decoding connection parameters.
- Instant connection without manual Device ID or 6-digit PIN typing.

## Architecture & Data Flow
1. **Pairing Payload Format (`QrPairingData`):**
   - Structured JSON: `{"type":"vrv_pairing","id":"684174","ip":"192.168.1.5","port":53211,"pin":"829104","signaling":"ws://..."}`
   - URI fallback: `vrvdesk://connect?id=684174&ip=192.168.1.5&port=53211&pin=829104`
2. **Components:**
   - `lib/src/services/qr_pairing_service.dart`: Payload serializer/deserializer with deep validation & error recovery.
   - `lib/src/widgets/qr_code_dialog.dart`: QR display modal (`qr_flutter`) for hosting & sharing.
   - `lib/src/views/qr_scanner_view.dart`: Camera viewfinder overlay (`mobile_scanner`) with flash toggle and cancel actions.
   - `lib/src/views/home_view.dart`: Integration with Device Card (Show QR) and Header (Scan QR button).

## Task Breakdown
- [ ] Task 1: `QrPairingService` Serializer, Parser & Unit Tests (`test/qr_pairing_test.dart`)
- [ ] Task 2: `QrCodeDialog` Display Widget & Tests (`lib/src/widgets/qr_code_dialog.dart`)
- [ ] Task 3: `QrScannerView` Scanner Camera Page (`lib/src/views/qr_scanner_view.dart`)
- [ ] Task 4: `HomeView` UI Integration (Scan Button & Show QR on Device Card)
- [ ] Task 5: Compilation, Unit/Widget Tests, Build APK & Emulator Verification
