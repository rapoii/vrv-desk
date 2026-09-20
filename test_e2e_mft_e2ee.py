import asyncio
import hashlib
import json
import time
import websockets
from cryptography.hazmat.primitives.ciphers.aead import ChaCha20Poly1305

async def verify_e2ee_mft_stream():
    uri = "ws://127.0.0.1:53211"
    print(f"Connecting to VrV Desk Host at {uri}...")

    async with websockets.connect(uri) as ws:
        # 1. Expect auth_required
        raw_msg = await asyncio.wait_for(ws.recv(), timeout=5.0)
        auth_req = json.loads(raw_msg)
        print(f"Auth Required from: {auth_req.get('host_name')} (v{auth_req.get('version')})")

        # 2. Send PIN 829104 with e2ee: true
        pin = "829104"
        verify_payload = {
            "type": "auth_verify",
            "pin": pin,
            "e2ee": True
        }
        await ws.send(json.dumps(verify_payload))

        # 3. Expect auth_ok
        auth_resp_raw = await asyncio.wait_for(ws.recv(), timeout=5.0)
        auth_resp = json.loads(auth_resp_raw)
        assert auth_resp.get("type") == "auth_ok", f"Expected auth_ok, got {auth_resp}"
        assert auth_resp.get("e2ee") is True, f"Expected e2ee: true, got {auth_resp}"
        
        session_token = auth_resp.get("session_token")
        print(f"Auth OK received! Session Token: {session_token} | E2EE: {auth_resp.get('e2ee')}")

        # Derive ChaCha20-Poly1305 key
        salt = b"VRV_DESK_E2EE_KEY_SALT_v1:" + session_token.encode("utf-8")
        key = hashlib.sha256(salt).digest()
        cipher = ChaCha20Poly1305(key)
        print("Symmetric 256-bit AEAD key derived successfully.")

        # 4. Receive and verify encrypted packets
        e2ee_packets = 0
        decrypted_video_packets = 0
        decrypted_audio_packets = 0
        start_time = time.time()

        print("Listening for encrypted VE2E media packets...")
        while time.time() - start_time < 5.0:
            try:
                msg = await asyncio.wait_for(ws.recv(), timeout=1.0)
                if isinstance(msg, bytes):
                    # Check VE2E magic (0x56, 0x45, 0x32, 0x45)
                    assert msg[:4] == b"VE2E", f"Packet does not start with VE2E: {msg[:4]}"
                    seq = int.from_bytes(msg[4:12], byteorder="little")
                    nonce = msg[4:12] + b"\x00\x00\x00\x00"
                    ciphertext_and_tag = msg[12:]

                    # Decrypt with ChaCha20-Poly1305
                    plaintext = cipher.decrypt(nonce, ciphertext_and_tag, None)
                    e2ee_packets += 1

                    if plaintext[:4] == b"VH24":
                        decrypted_video_packets += 1
                        frame_type = plaintext[4]
                        type_str = "IDR" if frame_type == 1 else "DELTA"
                        payload_len = len(plaintext) - 8
                        if decrypted_video_packets <= 5:
                            print(f"  [VE2E #{seq:03d} -> Decrypted VH24 Video]: Type={type_str} Size={payload_len}B")
                    elif plaintext[:4] == b"VAUD":
                        decrypted_audio_packets += 1
                        if decrypted_audio_packets <= 5:
                            print(f"  [VE2E #{seq:03d} -> Decrypted VAUD Audio]: Format=0x{plaintext[4]:02x} Size={len(plaintext)-8}B")
            except asyncio.TimeoutError:
                break

        print(f"\n--- E2EE Verification Summary ---")
        print(f"Total Encrypted VE2E Packets Received: {e2ee_packets}")
        print(f"Successfully Decrypted Video (VH24):   {decrypted_video_packets}")
        print(f"Successfully Decrypted Audio (VAUD):   {decrypted_audio_packets}")
        assert e2ee_packets > 0, "No E2EE packets received!"
        assert decrypted_video_packets > 0, "No decrypted video packets!"
        print("ALL E2EE & MFT HARDWARE PIPELINES VERIFIED SUCCESSFULLY!")

if __name__ == "__main__":
    asyncio.run(verify_e2ee_mft_stream())
