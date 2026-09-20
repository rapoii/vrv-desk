# VrV Desk Competitor Benchmark & Feature Gap Roadmap

> **Dokumen Riset Pasar & Rencana Pengembangan Fitur**  
> Sumber data benchmark: Audit langsung via MCP Playwright terhadap [RustDesk](https://github.com/rustdesk/rustdesk), [AnyDesk](https://anydesk.com/en/features), dan [TeamViewer Remote](https://www.teamviewer.com/en/products/remote/features/).

---

## 1. Matriks Perbandingan Fitur Komprehensif

| Fitur / Parameter | VrV Desk (v0.14.0) | RustDesk | AnyDesk | TeamViewer | Parsec / Moonlight |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **Model & Lisensi** | Open Source (Rust) | Open Source (AGPLv3) | Proprietary (SaaS) | Proprietary (SaaS) | Proprietary / FOSS |
| **Engine Video Capture** | DXGI Desktop Duplication | DirectX / Scrap | DeskRT (Proprietary) | Proprietary RDP/VNC | NVFBC / DXGI DDA |
| **Video Codecs** | Hardware H.264 (MFT) | VP8, VP9, AV1, H.264 | DeskRT, H.264/H.265 | Proprietary H.264 | H.264, HEVC, AV1 |
| **Audio Streaming** | WASAPI + Android 10+ Internal 48kHz Opus | WASAPI Loopback (PC Only) | WASAPI Loopback (PC Only) | Virtual Audio (PC Only) | Virtual Sink Loopback (PC Only) |
| **Bi-directional Clipboard** | ✅ **Ada (Auto Sync & Echo-Free)** | ✅ Ada (Teks & File) | ✅ Ada | ✅ Ada | ✅ Ada (Teks) |
| **File Transfer Manager** | ❌ *Belum Ada* | ✅ Ada (Dedicated UI) | ✅ Ada (Dual Panel) | ✅ Ada (Dual Panel) | ❌ Tidak Ada (Hanya P2P) |
| **Unattended Access** | ❌ Dynamic PIN Saja | ✅ Password Tetap + 2FA | ✅ Password Tetap + 2FA | ✅ Whitelist + Security | ✅ Akun Permanen |
| **Multi-Monitor Switcher** | ❌ Primary Screen Saja | ✅ Multi-Display Switch | ✅ Multi-Tab Display | ✅ Multi-Monitor Switch | ✅ Virtual / Phys Switch |
| **UAC Elevation & Service** | ❌ User Session Saja | ✅ Windows Service | ✅ Windows Service | ✅ Windows Service | ✅ System Service |
| **Virtual Display Driver** | ❌ Butuh Monitor Nyala | ✅ IddSampleDriver | ✅ Display Driver | ✅ Virtual Display | ✅ Parsec VDD / Idd |
| **Remote Reboot & Auto-Reconnect** | ❌ *Belum Ada* | ✅ Ada (+ Safe Mode) | ✅ Ada | ✅ Ada (+ Safe Mode) | ❌ Tidak Ada |
| **Privacy Mode (Screen Blanking)** | ❌ *Belum Ada* | ✅ Ada | ✅ Ada | ✅ Ada | ❌ Tidak Ada |
| **System Shortcuts (Ctrl+Alt+Del)**| ❌ *Belum Ada* | ✅ Ada | ✅ Ada | ✅ Ada | ❌ Terbatas |
| **In-Session Chat & Whiteboard** | ❌ *Belum Ada* | ✅ Ada | ✅ Ada | ✅ Ada | ❌ Tidak Ada |
| **Session Recording** | ❌ *Belum Ada* | ✅ Ada | ✅ Ada | ✅ Ada | ❌ Tidak Ada |
| **Web Client (Browser Remote)** | ❌ *Belum Ada* | ✅ Ada (WASM/WebRTC) | ✅ Ada (go.anydesk.com) | ✅ Ada (Web Client) | ✅ Ada (WebRTC) |
| **Remote Audio Android Host** | ✅ **Internal Audio 48kHz** | ❌ Bisu (Hanya Mic) | ❌ Bisu (Hanya Mic) | ❌ Bisu (Hanya Mic) | ❌ Client Saja |
| **Ukuran Installer Windows** | ✅ **~16 MB (Standalone)**| ~25 - 35 MB | ~5 MB (Zero-install) | ~50 - 90 MB | ~40 MB |
| **Konsumsi RAM Idle Host** | ✅ **~14 MB** | ~45 - 70 MB | ~35 - 50 MB | ~150 - 300 MB | ~60 - 90 MB |

---

## 2. Rincian Fitur yang Belum Ada di VrV Desk (Kelemahan & Target Peningkatan)

### Kategori A: Fitur Esensial Remote Desktop
1. **Bi-directional Clipboard Sync:**
   - Menyinkronkan teks, tautan, dan gambar yang di-copy di PC host agar langsung masuk ke clipboard Android/client secara instan, dan sebaliknya.
2. **File Transfer / File Manager Dedicated:**
   - Panel eksplorasi file dua sisi (Host vs Remote) untuk mengirim atau mengambil file/folder secara asinkron di latar belakang tanpa mengganggu streaming layar.
3. **Unattended Access (Password Permanen):**
   - Pengaturan password tetap yang disimpan secara aman (Argon2id/PBKDF2 hash) sehingga komputer kantor atau server pribadi dapat di-remote tanpa perlu ada operator di depan PC fisik.
4. **Multi-Monitor Switcher:**
   - Kemampuan mendeteksi dan berpindah antar monitor fisik (Display 1, Display 2) atau melihat seluruh monitor dalam satu tampilan gabungan (*All Monitors View*).

### Kategori B: Penanganan Sistem Operasi (OS Level)
5. **Windows Service Daemon (Bypass UAC & Lock Screen):**
   - Menjalankan background service di level `NT AUTHORITY\SYSTEM`. Menjaga sesi remote tetap hidup saat Windows berada di layar kunci (*Lock Screen*), setelah reboot, dan tidak freeze/black screen ketika muncul prompt administrator UAC (*User Account Control*).
6. **Remote Reboot & Auto-Reconnect:**
   - Perintah dari viewer untuk merestart komputer remote dan otomatis menyambungkan kembali koneksi saat OS Windows selesai booting (termasuk reboot ke *Safe Mode with Networking*).
7. **System Special Keys (Ctrl+Alt+Del, Win+L, Task Manager):**
   - Tombol khusus di toolbar viewer untuk menginjeksi kombinasi tombol sensitif Windows melalui Win32 SAS API (`SendSAS`) yang diblokir oleh injeksi standar.
8. **Privacy Mode / Screen Blanking:**
   - Mematikan atau membuat monitor fisik di PC target menjadi hitam pekat saat sesi remote sedang berlangsung agar privasi pengguna tidak diintip orang di ruangan host.

### Kategori C: Jaringan & Display Hardware
9. **Virtual Display Driver (Headless PC Support):**
   - Integrasi driver display virtual (seperti *IddSampleDriver* WDDM) agar PC tetap dapat di-stream dan dikontrol meskipun tidak tersambung ke monitor fisik atau monitor dimatikan.
10. **Wake-on-LAN (WoL):**
    - Mengirim magic packet UDP port 9 untuk membangunkan PC host yang berada dalam mode *Sleep* atau *Hibernation* dari jarak jauh.
11. **Web Client (Akses Langsung via Browser):**
    - Antarmuka web berbasis WebRTC + WASM sehingga pengguna dapat mengontrol PC target langsung dari browser (Chrome, Edge, Safari) tanpa perlu menginstal aplikasi viewer.

### Kategori D: Kolaborasi & Kontrol Sesi
12. **In-Session Chat & Whiteboard:**
    - Kotak obrolan teks real-time antara pengguna host dan operator client, serta fitur pena anotasi/gambar di atas layar untuk bimbingan teknis.
13. **Session Recording:**
    - Perekaman sesi remote langsung dari sisi viewer menjadi file MP4 lokal untuk keperluan dokumentasi atau audit kerja.
14. **Two-Way Audio / Remote Microphone:**
    - Mengirimkan suara mikrofon dari perangkat client ke PC host (audio komunikasi 2 arah).
15. **Address Book & Status Perangkat:**
    - Menyimpan daftar komputer yang sering diakses lengkap dengan indikator status online/offline secara real-time.

---

## 3. Rencana Implementasi Masa Depan (Actionable Roadmap)

### Phase 19: Bi-directional Clipboard Sync (`clipboard_sync` & `clipboard_text`) - [x] **Completed & Verified**
- [x] Win32 Clipboard sequence monitor + Echo loop prevention pada Rust (`host_service.rs` & `vrv_host.rs`).
- [x] Android Host bi-directional clipboard sync listener & broadcast pada `android_host_service.dart`.
- [x] Flutter Client automatic background clipboard sync timer, auto-sync toggle, dan echo prevention pada `mirror_view.dart`.
- [x] Verifikasi live e2e python test (`test_e2e_clipboard.py`) dan Flutter widget test suite (127/127 tests passing).

### Phase 20: Unattended Access (Static Password)
- [ ] Tambahkan konfigurasi `unattended_enabled` dan hash password `unattended_password_hash` (Argon2id) pada file config lokal `%APPDATA%\VrVDesk\config.toml`.
- [ ] Perbarui handshake auth pada `rust/src/pairing.rs` agar memvalidasi PIN dinamis atau Password Statis.
- [ ] Tambahkan toggle *"Enable Unattended Access"* dan form penyetelan password pada GUI `vrv_desk.rs` dan dialog Flutter.

### Phase 21: Dedicated File Transfer Manager
- [ ] Rancang protokol chunking file `FrameType::FileChunk = 0x06` (`VFIL`) dengan checksum Blake3/SHA256.
- [ ] Buat RPC untuk listing folder (`ListDir`, `DownloadFile`, `UploadFile`, `CancelTransfer`).
- [ ] Buat antarmuka dual-pane File Manager pada Flutter viewer dan Win32 GUI.

### Phase 22: Windows Service Daemon & UAC Elevation
- [ ] Buat binary service mandiri `vrv_service.exe` yang mendaftar ke Windows Service Control Manager (`CreateServiceW`).
- [ ] Hubungkan komunikasi IPC (Named Pipe) antara `vrv_service.exe` (SYSTEM) dan `vrv_desk.exe` (User Session).
- [ ] Gunakan `DuplicateTokenEx` dan injeksi `SendSAS` untuk mengeksekusi kontrol pada Secure Desktop (UAC & Lock Screen).

### Phase 23: Multi-Monitor Enumeration & Output Switcher
- [ ] Modifikasi `DxgiCapturer` agar mengenumerasi seluruh `IDXGIOutput` yang tersedia pada adapter grafis.
- [ ] Tambahkan tombol dropdown pemilihan display pada Viewer HUD (Display 1, Display 2, dll).
- [ ] Implementasikan dynamic rebind capture loop saat pengguna berpindah monitor tanpa memutus sesi streaming.

### Phase 24: Privacy Mode & System Shortcuts
- [ ] Implementasikan driver blank screen atau pemanggilan API DWM/GDI `SetDeviceGammaRamp` / low-level monitor power off.
- [ ] Tambahkan panel tombol cepat pada HUD: `Ctrl+Alt+Del`, `Win+L`, `Alt+Tab`, `Task Manager`.

### Phase 25: Virtual Display Driver (Headless PC)
- [ ] Bungkus open-source `IddSampleDriver` ke dalam paket installer VrV Desk.
- [ ] Buat opsi toggle *"Enable Virtual Headless Monitor"* jika tidak ada monitor fisik aktif yang terdeteksi oleh DXGI.
