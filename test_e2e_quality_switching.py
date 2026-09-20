import asyncio
import json
import subprocess
import sys
import time
import websockets

HOST_BIN = r"target\release\vrv_host.exe"
HOST_PIN = "123456"
HOST_PORT = 53211

async def test_quality_switching():
    print(f"[*] Spawning host: {HOST_BIN} with port {HOST_PORT}")
    proc = subprocess.Popen([
        HOST_BIN,
        "--pin", HOST_PIN,
        "--device-id", "TEST_QUALITY_001"
    ], stdout=subprocess.PIPE, stderr=subprocess.PIPE)

    # Allow socket to bind
    await asyncio.sleep(2.0)

    ws_url = f"ws://127.0.0.1:{HOST_PORT}"
    print(f"[*] Connecting WebSocket to {ws_url}...")

    try:
        async with websockets.connect(ws_url) as ws:
            # 1. Auth Handshake
            challenge_msg = await asyncio.wait_for(ws.recv(), timeout=3.0)
            challenge_data = json.loads(challenge_msg)
            print(f"[+] Received handshake challenge: {challenge_data.get('type')}")
            assert challenge_data.get("type") == "auth_required"

            # Send auth response
            auth_resp = {
                "type": "auth_verify",
                "pin": HOST_PIN
            }
            await ws.send(json.dumps(auth_resp))

            auth_ack_msg = await asyncio.wait_for(ws.recv(), timeout=3.0)
            auth_ack = json.loads(auth_ack_msg)
            print(f"[+] Handshake response: {auth_ack}")
            assert auth_ack.get("type") == "auth_ok"

            # 2. Receive initial frames
            frames_received = 0
            for _ in range(15):
                frame = await asyncio.wait_for(ws.recv(), timeout=2.0)
                if isinstance(frame, bytes):
                    frames_received += 1

            print(f"[+] Successfully received {frames_received} initial video frames")
            assert frames_received > 0, "Should have received initial frames"

            # 3. Test Switch to ECO (720p 30fps 1.2Mbps)
            print("\n[*] Sending set_quality: 'eco'...")
            await ws.send(json.dumps({"type": "set_quality", "profile": "eco"}))

            eco_ack = None
            start_t = time.time()
            while time.time() - start_t < 4.0:
                msg = await asyncio.wait_for(ws.recv(), timeout=1.5)
                if isinstance(msg, str):
                    parsed = json.loads(msg)
                    if parsed.get("type") == "quality_changed":
                        eco_ack = parsed
                        break

            print(f"[+] Received quality_changed response for ECO: {eco_ack}")
            assert eco_ack is not None, "Did not receive quality_changed ack for eco"
            assert eco_ack.get("profile") == "eco"
            assert eco_ack.get("height") <= 720
            assert eco_ack.get("bitrate_kbps") == 1200
            assert eco_ack.get("fps") == 30

            # 4. Test Switch to ULTRA (Native 60fps 6.0Mbps)
            print("\n[*] Sending set_quality: 'ultra'...")
            await ws.send(json.dumps({"type": "set_quality", "profile": "ultra"}))

            ultra_ack = None
            start_t = time.time()
            while time.time() - start_t < 4.0:
                msg = await asyncio.wait_for(ws.recv(), timeout=1.5)
                if isinstance(msg, str):
                    parsed = json.loads(msg)
                    if parsed.get("type") == "quality_changed":
                        ultra_ack = parsed
                        break

            print(f"[+] Received quality_changed response for ULTRA: {ultra_ack}")
            assert ultra_ack is not None, "Did not receive quality_changed ack for ultra"
            assert ultra_ack.get("profile") == "ultra"
            assert ultra_ack.get("bitrate_kbps") == 6000
            assert ultra_ack.get("fps") == 60

            # 5. Test Switch to BALANCED (1080p 60fps 2.5Mbps)
            print("\n[*] Sending set_quality: 'balanced'...")
            await ws.send(json.dumps({"type": "set_quality", "profile": "balanced"}))

            balanced_ack = None
            start_t = time.time()
            while time.time() - start_t < 4.0:
                msg = await asyncio.wait_for(ws.recv(), timeout=1.5)
                if isinstance(msg, str):
                    parsed = json.loads(msg)
                    if parsed.get("type") == "quality_changed":
                        balanced_ack = parsed
                        break

            print(f"[+] Received quality_changed response for BALANCED: {balanced_ack}")
            assert balanced_ack is not None, "Did not receive quality_changed ack for balanced"
            assert balanced_ack.get("profile") == "balanced"
            assert balanced_ack.get("height") <= 1080
            assert balanced_ack.get("bitrate_kbps") == 2500
            assert balanced_ack.get("fps") == 60

            # 6. Verify stream continues pumping frames after switches
            post_switch_frames = 0
            for _ in range(15):
                frame = await asyncio.wait_for(ws.recv(), timeout=2.0)
                if isinstance(frame, bytes):
                    post_switch_frames += 1

            print(f"\n[+] Stream health verified: Received {post_switch_frames} post-switch frames smoothly!")
            assert post_switch_frames > 0

            print("\n🎉 ALL QUALITY SWITCHING LIVE E2E CHECKS PASSED PERFECTLY! 🎉")

    finally:
        print("[*] Terminating host process...")
        proc.terminate()
        try:
            proc.wait(timeout=3)
        except subprocess.TimeoutExpired:
            proc.kill()
        print("[+] Host process terminated cleanly.")

if __name__ == "__main__":
    asyncio.run(test_quality_switching())
