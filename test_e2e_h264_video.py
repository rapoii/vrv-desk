import asyncio
import json
import time
import websockets

async def verify_h264_stream():
    uri = "ws://127.0.0.1:53211"
    print(f"Connecting to VrV Desk Host at {uri}...")

    async with websockets.connect(uri) as ws:
        # Step 1: Read auth_required
        msg = await asyncio.wait_for(ws.recv(), timeout=5.0)
        auth_req = json.loads(msg)
        print("Received auth prompt:", auth_req)
        assert auth_req.get("type") == "auth_required"

        # Step 2: Send PIN verification
        await ws.send(json.dumps({"type": "auth_verify", "pin": "958102"}))
        auth_res = json.loads(await asyncio.wait_for(ws.recv(), timeout=5.0))
        print("Auth response:", auth_res)
        assert auth_res.get("type") == "auth_ok"
        print("Authenticated successfully! Listening for H.264 video and Opus audio packets...")

        vh24_packets = []
        vaud_packets = []
        jpeg_packets = []
        start_time = time.time()

        # Listen for ~5 seconds of streaming
        while time.time() - start_time < 5.0 and len(vh24_packets) < 50:
            data = await asyncio.wait_for(ws.recv(), timeout=5.0)
            if isinstance(data, bytes):
                if data[:4] == b"VH24":
                    vh24_packets.append(data)
                elif data[:4] == b"VAUD":
                    vaud_packets.append(data)
                elif data[:2] == b"\xff\xd8":
                    jpeg_packets.append(data)

        elapsed = time.time() - start_time
        print(f"\n--- Stream Metrics over {elapsed:.2f}s ---")
        print(f"Total H.264 (VH24) packets received: {len(vh24_packets)}")
        print(f"Total Opus  (VAUD) packets received: {len(vaud_packets)}")
        print(f"Total JPEG  packets received: {len(jpeg_packets)}")

        assert len(vh24_packets) > 0, "No VH24 H.264 packets received!"
        
        # Analyze H.264 packets
        idr_count = 0
        delta_count = 0
        total_h264_bytes = sum(len(p) for p in vh24_packets)

        for i, pkt in enumerate(vh24_packets):
            magic = pkt[:4]
            frame_type = pkt[4]
            seq = (pkt[6] << 8) | pkt[7]
            nal_payload = pkt[8:]

            if frame_type == 1:
                idr_count += 1
            elif frame_type == 2:
                delta_count += 1

            if i < 5 or i == len(vh24_packets) - 1:
                print(f"  Pkt {i:02d}: Magic={magic.decode()} Type={'IDR' if frame_type == 1 else 'DELTA'} Seq={seq} PayloadSize={len(nal_payload)} bytes")

        avg_pkt_size = total_h264_bytes / len(vh24_packets)
        kbps = (total_h264_bytes * 8) / (elapsed * 1000)

        print(f"\n--- H.264 Compression Benchmark ---")
        print(f"Keyframes (IDR): {idr_count}")
        print(f"Delta frames (P): {delta_count}")
        print(f"Average H.264 packet size: {avg_pkt_size:.1f} bytes (vs ~80,000 bytes for JPEG)")
        print(f"H.264 Bitrate: {kbps:.2f} kbps ({kbps/8:.2f} KB/s)")
        print(f"Estimated bandwidth reduction: {((80000 - avg_pkt_size) / 80000) * 100:.1f}% savings!")

if __name__ == "__main__":
    asyncio.run(verify_h264_stream())
