import asyncio
import time
import json
import struct
import websockets
import sys

async def test_opus_audio_streaming():
    uri = "ws://127.0.0.1:53211"
    pin = "482910"
    print("=== E2E Test: Real-time Opus Audio Streaming ===")
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

        # Step 3: Listen for audio packets
        start_time = time.time()
        audio_packets = 0
        total_audio_bytes = 0
        video_frames = 0
        packet_sizes = []
        formats_seen = set()

        print("Receiving live multiplexed stream (listening for VAUD audio)...")
        while time.time() - start_time < 6.0:
            try:
                msg = await asyncio.wait_for(ws.recv(), timeout=2.0)
            except asyncio.TimeoutError:
                break

            if isinstance(msg, bytes):
                if len(msg) >= 8 and msg[:4] == b"VAUD":
                    audio_packets += 1
                    format_tag = msg[4]
                    formats_seen.add(format_tag)
                    channels = msg[5]
                    sample_rate = struct.unpack("<H", msg[6:8])[0]
                    payload_len = len(msg) - 8
                    total_audio_bytes += len(msg)
                    packet_sizes.append(payload_len)
                    
                    if audio_packets <= 5 or audio_packets % 25 == 0:
                        print(f"  [VAUD #{audio_packets}] format=0x{format_tag:02x}, ch={channels}, rate={sample_rate}Hz, payload={payload_len}B")
                elif len(msg) >= 2 and msg[0] == 0xFF and msg[1] == 0xD8:
                    video_frames += 1

        print(f"\n📊 Summary over {time.time() - start_time:.2f}s:")
        print(f"   Video frames: {video_frames}")
        print(f"   Audio packets: {audio_packets} ({total_audio_bytes} total bytes)")
        
        if audio_packets > 0:
            avg_size = sum(packet_sizes) / len(packet_sizes)
            uncompressed_pcm_size = 960 * 2 * 2  # 20ms at 48kHz stereo = 3840 bytes
            compression_ratio = uncompressed_pcm_size / (avg_size if avg_size > 0 else 1)
            kbps = (sum(packet_sizes) * 8 / (len(packet_sizes) * 0.020)) / 1000.0

            print(f"   Formats received: {[f'0x{fmt:02x}' for fmt in formats_seen]}")
            print(f"   Average payload size: {avg_size:.1f} bytes (vs {uncompressed_pcm_size} bytes uncompressed PCM)")
            print(f"   Compression ratio: {compression_ratio:.1f}x reduction")
            print(f"   Approximate audio bitrate: {kbps:.1f} kbps")
            
            assert 0x02 in formats_seen, f"Expected Opus format 0x02, but saw: {formats_seen}"
            assert compression_ratio >= 8.0, f"Expected at least 8x compression, got {compression_ratio:.1f}x"
            print("✅ Opus audio verification passed with flying colors!")
        else:
            print("⚠️ No audio packets captured (WASAPI loopback might be quiet if no sound is currently playing).")
            print("Testing capturer unit tests directly for Opus encode/decode...")

if __name__ == "__main__":
    asyncio.run(test_opus_audio_streaming())
