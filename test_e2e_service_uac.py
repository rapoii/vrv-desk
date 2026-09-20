import asyncio
import json
import os
import subprocess
import sys
import time
import websockets

async def test_websocket_commands():
    uri = "ws://127.0.0.1:53211"
    pin = "123456"
    print("[1] Connecting to Host at", uri)

    async with websockets.connect(uri, max_size=10*1024*1024) as ws:
        # Handshake
        challenge_raw = await asyncio.wait_for(ws.recv(), timeout=5.0)
        challenge = json.loads(challenge_raw)
        assert challenge.get("type") == "auth_required"

        await ws.send(json.dumps({"type": "auth_verify", "pin": pin, "e2ee": False}))
        auth_resp = json.loads(await asyncio.wait_for(ws.recv(), timeout=5.0))
        assert auth_resp.get("type") == "auth_ok"
        print("✅ Authenticated with Host")

        # 1. Query service_status
        await ws.send(json.dumps({"type": "service_status"}))
        raw = await asyncio.wait_for(ws.recv(), timeout=5.0)
        resp = json.loads(raw)
        assert resp.get("type") == "service_status_result", f"Unexpected: {resp}"
        print(f"✅ service_status received: installed={resp.get('installed')}, elevated={resp.get('elevated')}")

        # 2. Trigger system_sas (Ctrl+Alt+Del)
        await ws.send(json.dumps({"type": "system_sas"}))
        raw = await asyncio.wait_for(ws.recv(), timeout=5.0)
        resp = json.loads(raw)
        assert resp.get("type") == "system_sas_result", f"Unexpected: {resp}"
        print(f"✅ system_sas executed: success={resp.get('success')}")

        # 3. Trigger system_desktop_switch
        await ws.send(json.dumps({"type": "system_desktop_switch"}))
        raw = await asyncio.wait_for(ws.recv(), timeout=5.0)
        resp = json.loads(raw)
        assert resp.get("type") == "system_desktop_switch_result", f"Unexpected: {resp}"
        print(f"✅ system_desktop_switch received: success={resp.get('success')}")

def test_service_cli():
    service_bin = r"D:\Software\Hermes Workspace\projects\vrv-desk\target\release\vrv_service.exe"
    proc = subprocess.run([service_bin, "--status"], capture_output=True, text=True)
    assert proc.returncode == 0, f"service --status returned {proc.returncode}"
    status = json.loads(proc.stdout)
    assert "installed" in status and "running" in status and "elevated" in status
    print(f"✅ vrv_service CLI --status verified: {status}")

async def main():
    test_service_cli()

    host_bin = r"D:\Software\Hermes Workspace\projects\vrv-desk\target\release\vrv_host.exe"
    env = os.environ.copy()
    env["VRV_PIN"] = "123456"

    host_proc = subprocess.Popen([host_bin], env=env)
    time.sleep(2.5)

    try:
        await test_websocket_commands()
        print("\n🎉 ALL PHASE 22 SERVICE & UAC SYSTEM COMMANDS PASSED SUCCESSFULLY!")
    finally:
        host_proc.terminate()
        try:
            host_proc.wait(timeout=3)
        except Exception:
            host_proc.kill()

if __name__ == "__main__":
    asyncio.run(main())
