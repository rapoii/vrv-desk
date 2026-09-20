import asyncio
import json
import os
import subprocess
import sys
import time
import websockets

async def test_virtual_display_e2e():
    uri = "ws://127.0.0.1:53211"
    pin = "123456"

    print("[Phase 25 E2E] Spawning vrv_host.exe...")
    exe_path = os.path.abspath("target/release/vrv_host.exe")
    assert os.path.exists(exe_path), f"Host binary not found: {exe_path}"

    proc = subprocess.Popen(
        [exe_path, "--pin", pin, "--port", "53211"],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
    )

    try:
        time.sleep(1.5)

        print("[1] Connecting to live vrv_host on port 53211...")
        async with websockets.connect(uri) as ws:
            # 1. Auth Challenge
            challenge_msg = await ws.recv()
            if isinstance(challenge_msg, bytes):
                challenge_msg = challenge_msg.decode("utf-8", errors="ignore")
            challenge = json.loads(challenge_msg)
            assert challenge.get("type") == "auth_required"

            # 2. Auth Verify
            await ws.send(json.dumps({
                "type": "auth_verify",
                "pin": pin,
                "client_id": "test_phase25_client"
            }))

            auth_res = None
            for _ in range(30):
                raw = await ws.recv()
                if isinstance(raw, str):
                    resp = json.loads(raw)
                    if resp.get("type") == "auth_ok":
                        auth_res = resp
                        break
                    elif resp.get("type") == "auth_error":
                        raise AssertionError(f"Auth failed: {resp}")

            assert auth_res is not None
            print("[2] Successfully authenticated with PIN!")

            # 3. Query Virtual Display Status
            print("[3] Querying get_virtual_display_status...")
            await ws.send(json.dumps({
                "type": "get_virtual_display_status"
            }))

            vdd_status = None
            for _ in range(30):
                raw = await ws.recv()
                if isinstance(raw, str):
                    msg = json.loads(raw)
                    if msg.get("type") == "virtual_display_status_res":
                        vdd_status = msg
                        break

            assert vdd_status is not None, "Did not receive virtual_display_status_res"
            print(f"    Driver Installed: {vdd_status.get('driver_installed')}")
            print(f"    Driver Name: {vdd_status.get('driver_name')}")
            print(f"    Headless Mode: {vdd_status.get('is_headless')}")
            print(f"    Physical Monitors: {vdd_status.get('physical_monitor_count')}")
            print(f"    Supported Modes Count: {len(vdd_status.get('modes', []))}")
            assert len(vdd_status.get("modes", [])) >= 3, "Should provide at least 3 display modes"

            # 4. Request Driver Installation RPC
            print("[4] Testing install_virtual_display RPC...")
            await ws.send(json.dumps({
                "type": "install_virtual_display"
            }))

            install_res = None
            for _ in range(30):
                raw = await ws.recv()
                if isinstance(raw, str):
                    msg = json.loads(raw)
                    if msg.get("type") == "install_virtual_display_res":
                        install_res = msg
                        break

            assert install_res is not None, "Did not receive install_virtual_display_res"
            print(f"    Install RPC Result: success={install_res.get('success')}, message='{install_res.get('message')}'")

            # 5. Request Driver Uninstallation RPC
            print("[5] Testing uninstall_virtual_display RPC...")
            await ws.send(json.dumps({
                "type": "uninstall_virtual_display"
            }))

            uninstall_res = None
            for _ in range(30):
                raw = await ws.recv()
                if isinstance(raw, str):
                    msg = json.loads(raw)
                    if msg.get("type") == "uninstall_virtual_display_res":
                        uninstall_res = msg
                        break

            assert uninstall_res is not None, "Did not receive uninstall_virtual_display_res"
            print(f"    Uninstall RPC Result: success={uninstall_res.get('success')}, message='{uninstall_res.get('message')}'")

            print("\n🎉 ALL PHASE 25 VIRTUAL DISPLAY & HEADLESS E2E ASSERTIONS PASSED!")

    finally:
        print("Terminating vrv_host.exe...")
        proc.terminate()
        try:
            proc.wait(timeout=3)
        except subprocess.TimeoutExpired:
            proc.kill()

if __name__ == "__main__":
    asyncio.run(test_virtual_display_e2e())
