import asyncio
import json
import struct
import hashlib
import websockets
from cryptography.hazmat.primitives.ciphers.aead import ChaCha20Poly1305

PIN = "871188"
HOST = "ws://127.0.0.1:53211"

async def main():
    print(f"Connecting to {HOST}...")
    async with websockets.connect(HOST) as ws:
        # Step 1: Wait for auth_required
        msg = await ws.recv()
        print(f"Received from Android Host: {msg}")
        
        # Step 2: Send auth_verify
        auth_req = {
            "type": "auth_verify",
            "pin": PIN,
            "e2ee": True
        }
        await ws.send(json.dumps(auth_req))
        print(f"Sent auth_verify with PIN: {PIN}")
        
        # Step 3: Wait for auth_ok
        msg = await ws.recv()
        print(f"Received auth response: {msg}")
        resp = json.loads(msg)
        if resp.get("type") != "auth_ok":
            print(f"Auth failed: {resp}")
            return
            
        session_token = resp.get("session_token")
        print(f"Auth SUCCESS! Session token: {session_token[:10]}...")
        
        # Step 4: Derive ChaCha20-Poly1305 key
        salt = f"VRV_DESK_E2EE_KEY_SALT_v1:{session_token}".encode('utf-8')
        key = hashlib.sha256(salt).digest()
        cipher = ChaCha20Poly1305(key)
        
        send_seq = 0
        def encrypt_packet(plaintext_bytes: bytes) -> bytes:
            nonlocal send_seq
            seq_bytes = struct.pack('<Q', send_seq)
            nonce = seq_bytes + b'\x00' * 4
            # ChaCha20Poly1305 in python's cryptography returns ciphertext + 16-byte tag
            ct_and_tag = cipher.encrypt(nonce, plaintext_bytes, None)
            # Flutter E2eeTransport expects:
            # magic (4 bytes: 'VE2E') + seq (8 bytes) + ct + tag
            packet = b"VE2E" + seq_bytes + ct_and_tag
            send_seq += 1
            return packet

        # Command 1: Trigger RECENTS via globalAction
        print("\n--- Sending Command 1: 'RECENTS' (Overview screen) ---")
        cmd1 = json.dumps({"type": "shortcut", "action": "RECENTS"}).encode('utf-8')
        packet1 = encrypt_packet(cmd1)
        await ws.send(packet1)
        print("Command 1 sent successfully!")
        
        await asyncio.sleep(2.0)
        
        # Command 2: Trigger HOME via globalAction
        print("\n--- Sending Command 2: 'HOME' (Go to Home screen) ---")
        cmd2 = json.dumps({"type": "shortcut", "action": "HOME"}).encode('utf-8')
        packet2 = encrypt_packet(cmd2)
        await ws.send(packet2)
        print("Command 2 sent successfully!")
        
        await asyncio.sleep(2.0)

        # Command 3: Swipe Up (Open App Drawer from Home)
        print("\n--- Sending Command 3: Swipe Up from (0.5, 0.8) to (0.5, 0.2) ---")
        cmd3 = json.dumps({
            "type": "swipe",
            "x1": 0.5,
            "y1": 0.8,
            "x2": 0.5,
            "y2": 0.2,
            "duration": 300
        }).encode('utf-8')
        packet3 = encrypt_packet(cmd3)
        await ws.send(packet3)
        print("Command 3 sent successfully!")
        
        await asyncio.sleep(2.0)
        print("\nAll commands executed cleanly!")

if __name__ == "__main__":
    asyncio.run(main())
