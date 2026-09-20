# Session Viewer HUD & Network Watchdog Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`...[truncated]

**Goal:** Complete the remaining features specified in `docs/ARCHITECTURE.md` (Section 5.2 Item 2 and Section 7 Item 1):
1. **Session Viewer Diagnostic HUD & Controls:** Realtime metrics (Latency, FPS, Bitrate, Loss), Input Switcher (Trackpad mode vs Direct Touch), Quality Switcher (Eco, Balanced, Ultra), and End Session quick confirmation.
2. **Network Watchdog & Auto-Reconnect:** 3.0s buffer timeout detection, transparent *"Mencoba menghubungkan kembali..."* overlay with exponential backoff (1s, 2s, 4s), and state preservation.

---

### Task 1: Diagnostic Metrics & Session Viewer HUD

**Files:**
- Modify: `lib/src/views/mirror_view.dart`
- Modify: `test/mirror_view_toolbar_test.dart`

- [x] **Step 1: Write widget tests for HUD diagnostics, Quality Switcher, Input Switcher, and End Session confirmation in `test/mirror_view_toolbar_test.dart`**
- [x] **Step 2: Run tests to verify failure**
- [x] **Step 3: Implement Diagnostic Metrics Tracker (FPS, Bitrate, Latency, Loss), Collapsible HUD Pill, Quality Switcher, Trackpad Input mode, and End Session Confirmation Dialog in `MirrorView`**
- [x] **Step 4: Run tests to verify they pass**
- [x] **Step 5: Commit**

---

### Task 2: Network Watchdog & Auto-Reconnect Overlay

**Files:**
- Modify: `lib/src/views/mirror_view.dart`
- Create / Modify: `test/network_watchdog_test.dart`

- [x] **Step 1: Write widget tests for 3.0s buffer timeout and *"Mencoba menghubungkan kembali..."* overlay with exponential backoff in `test/network_watchdog_test.dart`**
- [x] **Step 2: Run tests to verify failure**
- [x] **Step 3: Implement 3.0s watchdog timer, exponential backoff (1s, 2s, 4s), and auto-reconnect overlay in `MirrorView`**
- [x] **Step 4: Run tests to verify they pass**
- [x] **Step 5: Full regression test & push**
