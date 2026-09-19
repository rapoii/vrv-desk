### What's New in v0.8.0:
- **DirectX 11 DXGI Desktop Duplication GPU Capture (Task 6 Master Plan)**:
  - **Hardware Acceleration via DXGI 1.2 (`IDXGIOutputDuplication`)**: Replaced CPU-bound GDI BitBlt with direct GPU VRAM capture for ultra-low latency (<15ms) and smooth 60 FPS video streaming.
  - **Smart Delta Video Loop**: The capture engine detects static frames (`DXGI_ERROR_WAIT_TIMEOUT`) and avoids wasteful re-encoding and network flooding when pixels haven't changed, slashing host CPU usage to under 5%.
  - **Resilient Hybrid Capturer Architecture**: Automatically tests and uses hardware DXGI GPU capture on native Windows, with graceful fallback to GDI BitBlt when running on virtual machines, RDP, or non-accelerated sessions.
  - **Dynamic Engine Status Reporting**: Real-time console diagnostics show whether the host session is accelerated via DirectX 11 DXGI or falling back to GDI.
