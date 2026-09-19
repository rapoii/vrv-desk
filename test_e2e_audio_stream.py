#!/usr/bin/env python3
"""
VrV Desk - E2E Audio & Video Streaming Verification Script
Connects to vrv_host, performs PIN handshake, and verifies that both
JPEG video frames and VAUD PCM audio packets are received concurrently.
"""

import asyncio
import json
import struct
import sys
import websockets

async def test_audio_video_stream(host_url="ws://127.0.0.1:53211", pin="482910"):
    print(f"[*] Connecting to {host_url}...")
    async with websockets.connect(host_url) as ws:
        print("[*] Connected! Waiting for auth_required...")

        # 1. Auth Handshake
        auth_req_raw = await asyncio.wait_for(ws.recv(), timeout=5.0)
        auth_req = json.loads(auth_req_raw)
        assert auth_req.get("type") == "auth_required", f"Unexpected: {auth_req}"
        print(f"[✓] Received auth_required. Sending PIN: {pin}")

        verify_payload = json.dumps({"type": "auth_verify", "pin": pin})
        await ws.send(verify_payload)

        auth_resp_raw = await asyncio.wait_for(ws.recv(), timeout=5.0)
        auth_resp = json.loads(auth_resp_raw)
        assert auth_resp.get("type") == "auth_ok", f"Auth failed: {auth_resp}"
        print(f"[✓] Authenticated successfully: token={auth_resp.get('session_token')}")

        # 2. Receive frames and audio packets
        video_count = 0
        audio_count = 0
        total_audio_bytes = 0

        print("[*] Listening for concurrent video and audio packets for 5 seconds...")
        start_time = asyncio.get_event_loop().time()

        while asyncio.get_event_loop().time() - start_time < 5.0:
            try:
                msg = await asyncio.wait_for(ws.recv(), timeout=2.0)
            except asyncio.TimeoutError:
                break

            if isinstance(msg, bytes):
                if len(msg) >= 2 and msg[0] == 0xFF and msg[1] == 0xD8:
                    video_count += 1
                elif len(msg) >= 8 and msg[:4] == b"VAUD":
                    audio_count += 1
                    format_tag = msg[4]
                    channels = msg[5]
                    sample_rate = struct.unpack("<H", msg[6:8])[0]
                    pcm_data = msg[8:]
                    total_audio_bytes += len(pcm_data)
                    if audio_count == 1:
                        print(f"[✓] First VAUD Audio Packet received: format={format_tag} (PCM_S16LE), channels={channels}, sample_rate={sample_rate}Hz, chunk_size={len(pcm_data)} bytes")
            elif isinstance(msg, str):
                print(f"[i] Text message received: {msg[:60]}")

        print(f"\n==========================================")
        print(f"📊 Stream Statistics:")
        print(f"   Video Frames:  {video_count}")
        print(f"   Audio Packets: {audio_count}")
        print(f"   Audio Bytes:   {total_audio_bytes} bytes")
        print(f"==========================================")

        assert video_count > 0, "No video frames received!"
        assert audio_count > 0, "No audio packets received!"
        print("\n🎉 ALL AUDIO & VIDEO STREAMING TESTS PASSED!")

if __name__ == "__main__":
    url = sys.argv[1] if len(sys.argv) > 1 else "ws://127.0.0.1:53211"
    pin = sys.argv[2] if len(sys.argv) > 2 else "482910"
    try:
        asyncio.run(test_audio_video_stream(url, pin))
    except Exception as e:
        print(f"[-] Test failed: {e}")
        sys.exit(1)
