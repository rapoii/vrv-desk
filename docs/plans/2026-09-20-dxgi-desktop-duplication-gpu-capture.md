# Implementation Plan: DirectX 11 DXGI Desktop Duplication GPU Capture (Phase 8 / Task 6)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement hardware-accelerated DirectX 11 DXGI Desktop Duplication (`IDXGIOutputDuplication`) in Rust to capture screen frames directly from GPU VRAM with sub-15ms latency and zero-copy dirty region detection, with seamless GDI BitBlt fallback when running in virtualized or non-DirectX environments.

**Architecture:** A native Windows capture driver (`platform/windows_capture.rs`) creates a Direct3D 11 device and duplicates the primary display output. Frames are acquired from GPU memory into a staging texture, extracted as BGRA8 pixels, and encoded to JPEG for WebSocket transmission. A unified `ScreenCapturer` wraps DXGI as the primary engine with automatic GDI fallback.

**Tech Stack:**
- Direct3D 11 (`ID3D11Device`, `ID3D11DeviceContext`, `ID3D11Texture2D`)
- DXGI 1.2 (`IDXGIDevice`, `IDXGIAdapter`, `IDXGIOutput1`, `IDXGIOutputDuplication`)
- `image` crate (fast JPEG encoder) & `fast_image_resize`
- Rust 2021 edition

**Spec:** Task 6 in `docs/IMPLEMENTATION_PLAN.md` & Section 2.1 in `docs/ARCHITECTURE.md`.

## Global Constraints
- Primary engine: DirectX 11 DXGI Desktop Duplication API.
- Fallback engine: GDI BitBlt if D3D11/DXGI init fails or access is lost and unrecoverable.
- Latency target: Sub-15ms frame acquisition.
- Clean shutdown: Proper release of COM interfaces (`ReleaseFrame`, `Unmap`, drop semantics).
- Backwards compatibility: Emits binary JPEG frames (`0xFF, 0xD8`) identical to current wire protocol so mobile clients require zero breaking changes.

---

## Tasks Breakdown

- [ ] **Task 1: Rust DXGI Desktop Duplication Engine & Fallback Architecture** (`rust/src/platform/windows_capture.rs`, `rust/src/lib.rs`, `rust/tests/dxgi_capture_test.rs`).
- [ ] **Task 2: Host Daemon Integration & Smart Delta Video Loop** (`rust/src/bin/vrv_host.rs`).
- [ ] **Task 3: Integration, E2E Verification & Release v0.8.0** (`test_e2e_dxgi.py`, APK builds, emulator live test, GitHub Release).
