import asyncio
import json
import websockets
import subprocess
import time
import sys

PORT = 53211
TEST_PIN = "654321"

async def test_auth_workflow():
    uri = f"ws://127.0.0.1:{PORT}"
    
    print("--- 1. Testing Invalid PIN Lockout (Client 1) ---")
    async with websockets.connect(uri) as ws:
        # Expect auth_required
        msg = await asyncio.wait_for(ws.recv(), timeout=5.0)
        data = json.loads(msg)
        print(f"Received from host: {data}")
        assert data.get("type") == "auth_required", f"Expected auth_required, got {data}"
        
        # Send wrong PIN 1
        await ws.send(json.dumps({"type": "auth_verify", "pin": "000000"}))
        resp1 = json.loads(await asyncio.wait_for(ws.recv(), timeout=5.0))
        print(f"Wrong PIN 1 response: {resp1}")
        assert resp1.get("type") == "auth_failed"
        assert resp1.get("remaining_attempts") == 2
        
        # Send wrong PIN 2
        await ws.send(json.dumps({"type": "auth_verify", "pin": "111111"}))
        resp2 = json.loads(await asyncio.wait_for(ws.recv(), timeout=5.0))
        print(f"Wrong PIN 2 response: {resp2}")
        assert resp2.get("type") == "auth_failed"
        assert resp2.get("remaining_attempts") == 1
        
        # Send wrong PIN 3 (Lockout)
        await ws.send(json.dumps({"type": "auth_verify", "pin": "222222"}))
        resp3 = json.loads(await asyncio.wait_for(ws.recv(), timeout=5.0))
        print(f"Wrong PIN 3 response: {resp3}")
        assert resp3.get("type") == "auth_failed"
        assert resp3.get("remaining_attempts") == 0
        
        # Socket should be closed now
        try:
            extra = await asyncio.wait_for(ws.recv(), timeout=2.0)
            print(f"Unexpected extra message: {extra}")
            assert False, "Socket should have closed after 3 failed attempts"
        except (websockets.exceptions.ConnectionClosed, asyncio.TimeoutError):
            print("[✓] Lockout verified: connection closed after 3 failed attempts!")

    print("\n--- 2. Testing Valid PIN Authentication (Client 2) ---")
    async with websockets.connect(uri) as ws:
        # Expect auth_required
        msg = await asyncio.wait_for(ws.recv(), timeout=5.0)
        data = json.loads(msg)
        print(f"Received from host: {data}")
        assert data.get("type") == "auth_required"
        
        # Send valid PIN
        print(f"Sending valid PIN: {TEST_PIN}")
        await ws.send(json.dumps({"type": "auth_verify", "pin": TEST_PIN}))
        resp = json.loads(await asyncio.wait_for(ws.recv(), timeout=5.0))
        print(f"Auth response: {resp}")
        assert resp.get("type") == "auth_ok", f"Expected auth_ok, got {resp}"
        assert "session_token" in resp, "Expected session_token in auth_ok"
        print(f"[✓] Authenticated! Session Token: {resp['session_token']}")
        
        # Now host must stream binary JPEG frames
        print("Waiting for live desktop stream frame...")
        frame = await asyncio.wait_for(ws.recv(), timeout=5.0)
        assert isinstance(frame, bytes), f"Expected binary frame, got {type(frame)}"
        assert len(frame) > 1000, f"Frame too small: {len(frame)} bytes"
        is_jpeg = frame.startswith(b'\xff\xd8')
        print(f"[✓] Received live frame of size: {len(frame)} bytes (Valid JPEG: {is_jpeg})")
        
        # Test input after auth
        await ws.send(json.dumps({"type": "touch_tap", "x": 0.5, "y": 0.5}))
        print("[✓] Sent post-auth touch input event")

    print("\n🎉 ALL E2E AUTH HANDSHAKE TESTS PASSED!")

if __name__ == "__main__":
    asyncio.run(test_auth_workflow())
