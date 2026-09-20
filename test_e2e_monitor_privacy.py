import asyncio
import json
import os
import subprocess
import sys
import time
import websockets

async def test_monitor_privacy_e2e():
    uri = "ws://127.0.0.1:53211"
    pin = "123456"

    print("[1] Connecting to live vrv_host.exe on port 53211...")
    async with websockets.connect(uri) as ws:
        # Step 1: Handshake
        msg = await ws.recv()
        if isinstance(msg, bytes):
            msg = msg.decode('utf-8', errors='ignore')
        challenge = json.loads(msg)
        assert challenge.get("type") == "auth_required"

        await ws.send(json.dumps({
            "type": "auth_verify",
            "pin": pin,
            "e2ee": False
        }))

        auth_res = json.loads(await ws.recv())
        assert auth_res.get("type") == "auth_ok", f"Auth failed: {auth_res}"
        print("[2] Successfully authenticated with PIN!")

        # Step 2: Query Multi-Monitor Enumeration
        print("[3] Requesting monitor enumeration (get_monitors)...")
        await ws.send(json.dumps({"type": "get_monitors"}))
        
        # Loop until monitors_list is received (ignoring video/audio packets)
        monitors_data = None
        for _ in range(20):
            res = await ws.recv()
            if isinstance(res, str):
                parsed = json.loads(res)
                if parsed.get("type") == "monitors_list":
                    monitors_data = parsed
                    break

        assert monitors_data is not None, "Did not receive monitors_list"
        monitors = monitors_data.get("monitors", [])
        assert len(monitors) >= 1, "Must have at least 1 monitor"
        m0 = monitors[0]
        print(f"    Found {len(monitors)} monitor(s): {m0['name']} ({m0['width']}x{m0['height']}, primary={m0['is_primary']})")
        assert m0["width"] > 0 and m0["height"] > 0

        # Step 3: Switch Monitor (index 0)
        print("[4] Requesting switch_monitor to index 0...")
        await ws.send(json.dumps({"type": "switch_monitor", "index": 0}))
        switch_res = None
        for _ in range(20):
            res = await ws.recv()
            if isinstance(res, str):
                parsed = json.loads(res)
                if parsed.get("type") == "switch_monitor_res":
                    switch_res = parsed
                    break

        assert switch_res is not None, "Did not receive switch_monitor_res"
        print(f"    Switch res object: {switch_res}")
        assert switch_res.get("success") is True
        print(f"    Switch monitor success: {switch_res['width']}x{switch_res['height']}")

        # Step 4: Toggle Privacy Mode (set false for safety)
        print("[5] Requesting privacy mode toggle (set_privacy_mode = false)...")
        await ws.send(json.dumps({"type": "set_privacy_mode", "enabled": False}))
        priv_res = None
        for _ in range(20):
            res = await ws.recv()
            if isinstance(res, str):
                parsed = json.loads(res)
                if parsed.get("type") == "privacy_mode_res":
                    priv_res = parsed
                    break

        assert priv_res is not None, "Did not receive privacy_mode_res"
        assert priv_res.get("success") is True
        assert priv_res.get("enabled") is False
        print("    Privacy mode response: OK")

        # Step 5: System Action (safe abort_shutdown)
        print("[6] Requesting system action (abort_shutdown)...")
        await ws.send(json.dumps({"type": "system_action", "action": "abort_shutdown"}))
        action_res = None
        for _ in range(20):
            res = await ws.recv()
            if isinstance(res, str):
                parsed = json.loads(res)
                if parsed.get("type") == "system_action_res":
                    action_res = parsed
                    break

        assert action_res is not None, "Did not receive system_action_res"
        assert action_res.get("action") == "abort_shutdown"
        print(f"    System action response: {action_res.get('message')}")

    print("\n🎉 ALL PHASE 23 & 24 E2E ASSERTIONS PASSED!")

def main():
    host_exe = os.path.abspath("target/release/vrv_host.exe")
    print(f"Spawning host: {host_exe}")
    proc = subprocess.Popen(
        [host_exe, "--pin", "123456"],
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT
    )
    time.sleep(2)
    try:
        asyncio.run(test_monitor_privacy_e2e())
    finally:
        print("Terminating host...")
        proc.terminate()
        proc.kill()
        proc.wait()

if __name__ == "__main__":
    main()
