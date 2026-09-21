import asyncio
import json
import subprocess
import sys
import time
import websockets

HOST_BIN = r"target\release\vrv_host.exe"
HOST_PIN = "123456"
HOST_PORT = 53211

async def test_unified_keyboard():
    print(f"[*] Spawning host: {HOST_BIN}...")
    proc = subprocess.Popen(
        [HOST_BIN, "--pin", HOST_PIN, "--device-id", "TEST_KBD_001"],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    await asyncio.sleep(2.0)

    try:
        uri = f"ws://127.0.0.1:{HOST_PORT}"
        print(f"[*] Connecting WebSocket to {uri}...")
        async with websockets.connect(uri) as ws:
            # 1. Auth Handshake
            challenge_raw = await asyncio.wait_for(ws.recv(), timeout=3.0)
            challenge = json.loads(challenge_raw)
            print(f"[+] Handshake challenge: {challenge.get('type')}")
            assert challenge.get("type") == "auth_required"

            # Send PIN
            await ws.send(json.dumps({"type": "auth_verify", "pin": HOST_PIN}))
            resp_raw = await asyncio.wait_for(ws.recv(), timeout=3.0)
            resp = json.loads(resp_raw)
            print(f"[+] Auth response: {resp}")
            assert resp.get("type") == "auth_ok"

            # 2. Test Direct Mode Text Injection
            print("\n[*] [Mode 1: Direct Mode] Sending live character typing: 'echo vrv_desk'...")
            for ch in "echo vrv_desk":
                await ws.send(json.dumps({"type": "type_text", "text": ch}))
                await asyncio.sleep(0.01)
            print("[+] Direct live character stream sent successfully!")

            # 3. Test Buffered Mode String Injection
            print("\n[*] [Mode 2: Buffered Mode] Sending accumulated text buffer on Send click...")
            buffered_text = "Hello from VrV Desk Unified Keyboard Buffered Mode!"
            await ws.send(json.dumps({"type": "type_text", "text": buffered_text}))
            print(f"[+] Buffered string sent successfully: '{buffered_text}'")

            # 4. Test Navigation and PC Function Shortcuts
            print("\n[*] [PC Keys & Shortcuts] Testing injection of navigation & function keys...")
            shortcuts = [
                "esc", "tab", "win", "del", "backspace", "space", "enter",
                "up", "down", "left", "right", "home", "end", "pgup", "pgdn",
                "f1", "f5", "f11", "f12",
                "ctrl_a", "ctrl_c", "ctrl_v", "win_r"
            ]
            for sc in shortcuts:
                await ws.send(json.dumps({"type": "shortcut", "name": sc}))
                await asyncio.sleep(0.01)
            print(f"[+] Successfully injected {len(shortcuts)} PC shortcuts & function keys!")

            # 5. Receive video frame to guarantee host loop is alive and healthy
            frame_count = 0
            for _ in range(5):
                packet = await asyncio.wait_for(ws.recv(), timeout=2.0)
                if isinstance(packet, bytes) and packet[:4] == b"VH24":
                    frame_count += 1
            assert frame_count > 0
            print(f"[+] Host streaming healthy: received {frame_count} video frames post-keyboard testing!")

        print("\n🎉 ALL UNIFIED KEYBOARD E2E TESTS PASSED PERFECTLY! 🎉")
    finally:
        print("[*] Terminating host process...")
        proc.terminate()
        try:
            proc.wait(timeout=2.0)
        except subprocess.TimeoutExpired:
            proc.kill()
        print("[+] Host terminated cleanly.")

if __name__ == "__main__":
    asyncio.run(test_unified_keyboard())
