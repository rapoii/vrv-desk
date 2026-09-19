import asyncio
import time
import json
import struct
import websockets
import subprocess
import sys
import os

async def test_dxgi_streaming():
    uri = "ws://127.0.0.1:53211"
    pin = "482910"
    print(f"=== E2E Test: DirectX 11 DXGI GPU Video Streaming ===")
    print(f"Connecting to {uri} with PIN {pin}...")

    async with websockets.connect(uri) as ws:
        # Step 1: Wait for auth_required
        msg = await ws.recv()
        data = json.loads(msg)
        assert data.get("type") == "auth_required", f"Expected auth_required, got {data}"
        print("✅ Received auth_required challenge")

        # Step 2: Send auth_verify
        await ws.send(json.dumps({"type": "auth_verify", "pin": pin}))
        msg2 = await ws.recv()
        data2 = json.loads(msg2)
        assert data2.get("type") == "auth_ok", f"Expected auth_ok, got {data2}"
        print("✅ Authenticated successfully (auth_ok)!")

        # Step 3: Receive video frames for 5 seconds
        start_time = time.time()
        frame_count = 0
        total_bytes = 0
        durations = []
        last_frame_time = start_time

        print("Receiving live GPU desktop frames...")
        while time.time() - start_time < 5.0:
            msg = await asyncio.wait_for(ws.recv(), timeout=3.0)
            if isinstance(msg, bytes):
                # Check for JPEG magic bytes 0xFF, 0xD8
                if len(msg) >= 2 and msg[0] == 0xFF and msg[1] == 0xD8:
                    now = time.time()
                    durations.append(now - last_frame_time)
                    last_frame_time = now
                    frame_count += 1
                    total_bytes += len(msg)
                elif len(msg) >= 4 and msg[:4] == b"VAUD":
                    # Audio packet multiplexed, ignore for video test
                    pass

        fps = frame_count / (time.time() - start_time)
        print(f"✅ Received {frame_count} video frames ({total_bytes} bytes) in 5.0s")
        print(f"📊 Average FPS: {fps:.1f} FPS")
        assert frame_count >= 15, f"Expected at least 15 frames, got {frame_count}"

    print("=== DXGI E2E Verification Passed! ===")

if __name__ == "__main__":
    asyncio.run(test_dxgi_streaming())
