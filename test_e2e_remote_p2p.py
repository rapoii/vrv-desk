import asyncio
import json
import websockets
import sys

SIGNAL_URI = "ws://127.0.0.1:53212"
TARGET_DEVICE_ID = "849201"
TEST_PIN = "888999"

async def test_remote_p2p():
    print(f"[*] Connecting to Signaling Broker at {SIGNAL_URI}...")
    async with websockets.connect(SIGNAL_URI) as ws:
        print("[✓] Connected to Signaling Broker!")
        
        # 1. Request connection to target host
        connect_req = {
            "type": "connect_request",
            "target_id": TARGET_DEVICE_ID,
            "client_name": "Test-Client-Remote"
        }
        await ws.send(json.dumps(connect_req))
        print(f"[*] Sent connect_request for target: {TARGET_DEVICE_ID}")
        
        # 2. Wait for auth_required from bridged host session
        resp_raw = await asyncio.wait_for(ws.recv(), timeout=5.0)
        resp = json.loads(resp_raw)
        print(f"[✓] Received message from bridged host: {resp}")
        assert resp.get("type") == "auth_required", f"Expected auth_required, got {resp}"
        
        # 3. Send PIN verification
        auth_msg = {
            "type": "auth_verify",
            "pin": TEST_PIN
        }
        await ws.send(json.dumps(auth_msg))
        print(f"[*] Sent auth_verify with PIN: {TEST_PIN}")
        
        # 4. Receive auth_ok
        auth_ok_raw = await asyncio.wait_for(ws.recv(), timeout=5.0)
        auth_ok = json.loads(auth_ok_raw)
        print(f"[✓] Received auth confirmation: {auth_ok}")
        assert auth_ok.get("type") == "auth_ok", f"Expected auth_ok, got {auth_ok}"
        
        # 5. Receive live JPEG frame
        frame = await asyncio.wait_for(ws.recv(), timeout=5.0)
        assert isinstance(frame, bytes), "Expected binary frame"
        assert frame.startswith(b'\xff\xd8'), "Expected valid JPEG header"
        print(f"[✓] Received live frame through Remote Signaling Broker: {len(frame)} bytes!")
        
        # 6. Send input event
        input_event = {
            "type": "touch_tap",
            "x": 0.5,
            "y": 0.5
        }
        await ws.send(json.dumps(input_event))
        print("[✓] Sent remote touch event through Signaling Broker!")
        
    print("\n🎉 ALL REMOTE P2P SIGNALING & AUTH TESTS PASSED!")

if __name__ == "__main__":
    asyncio.run(test_remote_p2p())
