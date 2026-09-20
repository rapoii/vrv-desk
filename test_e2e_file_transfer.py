import asyncio
import base64
import json
import os
import subprocess
import sys
import time
import websockets

async def main():
    uri = "ws://127.0.0.1:53211"
    print("Testing VrV Desk File Transfer E2E...")

    pin = "778899"
    cmd = [r"D:\Software\Hermes Workspace\projects\vrv-desk\target\release\vrv_host.exe"]
    env = os.environ.copy()
    env["VRV_PIN"] = pin

    proc = subprocess.Popen(cmd, env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    time.sleep(1.5)

    try:
        async with websockets.connect(uri) as ws:
            # 1. Auth challenge handshake
            challenge_raw = await asyncio.wait_for(ws.recv(), timeout=5.0)
            challenge = json.loads(challenge_raw)
            assert challenge.get("type") == "auth_required"

            # Send auth_verify
            await ws.send(json.dumps({"type": "auth_verify", "pin": pin, "e2ee": False}))
            auth_ok_raw = await asyncio.wait_for(ws.recv(), timeout=5.0)
            auth_ok = json.loads(auth_ok_raw)
            assert auth_ok.get("type") == "auth_ok"
            print("✅ Authenticated with Host")

            # 2. Test fs_list
            await ws.send(json.dumps({"type": "fs_list", "id": "list_1", "path": "."}))
            while True:
                msg = await asyncio.wait_for(ws.recv(), timeout=5.0)
                if isinstance(msg, str):
                    data = json.loads(msg)
                    if data.get("id") == "list_1":
                        assert data.get("type") == "fs_list_resp"
                        entries = data.get("entries", [])
                        assert len(entries) > 0
                        print(f"✅ fs_list succeeded: found {len(entries)} entries")
                        break

            # 3. Test fs_mkdir
            test_dir = os.path.abspath("test_e2e_transfer_folder")
            await ws.send(json.dumps({"type": "fs_mkdir", "id": "mkdir_1", "path": test_dir}))
            while True:
                msg = await asyncio.wait_for(ws.recv(), timeout=5.0)
                if isinstance(msg, str):
                    data = json.loads(msg)
                    if data.get("id") == "mkdir_1":
                        assert data.get("type") == "fs_action_resp"
                        assert data.get("success") is True
                        print(f"✅ fs_mkdir succeeded: created {test_dir}")
                        break

            # 4. Test fs_write_chunk (Upload)
            test_file = os.path.join(test_dir, "test_file.txt")
            payload = b"VrV Desk Dedicated File Transfer Chunk Test 2026!"
            payload_b64 = base64.b64encode(payload).decode("utf-8")

            await ws.send(json.dumps({
                "type": "fs_write_chunk",
                "id": "write_1",
                "path": test_file,
                "offset": 0,
                "data_b64": payload_b64,
                "eof": True
            }))
            while True:
                msg = await asyncio.wait_for(ws.recv(), timeout=5.0)
                if isinstance(msg, str):
                    data = json.loads(msg)
                    if data.get("id") == "write_1":
                        assert data.get("type") == "fs_write_resp"
                        assert data.get("success") is True
                        assert data.get("bytes_written") == len(payload)
                        print(f"✅ fs_write_chunk succeeded: uploaded {len(payload)} bytes")
                        break

            # 5. Test fs_read_chunk (Download)
            await ws.send(json.dumps({
                "type": "fs_read_chunk",
                "id": "read_1",
                "path": test_file,
                "offset": 0,
                "length": 65536
            }))
            while True:
                msg = await asyncio.wait_for(ws.recv(), timeout=5.0)
                if isinstance(msg, str):
                    data = json.loads(msg)
                    if data.get("id") == "read_1":
                        assert data.get("type") == "fs_read_resp"
                        recv_b64 = data.get("data_b64")
                        recv_bytes = base64.b64decode(recv_b64)
                        assert recv_bytes == payload
                        assert data.get("eof") is True
                        print(f"✅ fs_read_chunk succeeded: downloaded and verified {len(recv_bytes)} bytes")
                        break

            # 6. Test fs_delete
            await ws.send(json.dumps({
                "type": "fs_delete",
                "id": "del_1",
                "path": test_file,
                "is_dir": False
            }))
            while True:
                msg = await asyncio.wait_for(ws.recv(), timeout=5.0)
                if isinstance(msg, str):
                    data = json.loads(msg)
                    if data.get("id") == "del_1":
                        assert data.get("type") == "fs_action_resp"
                        assert data.get("success") is True
                        print("✅ fs_delete file succeeded")
                        break

            # Delete folder
            await ws.send(json.dumps({
                "type": "fs_delete",
                "id": "del_2",
                "path": test_dir,
                "is_dir": True
            }))
            while True:
                msg = await asyncio.wait_for(ws.recv(), timeout=5.0)
                if isinstance(msg, str):
                    data = json.loads(msg)
                    if data.get("id") == "del_2":
                        assert data.get("type") == "fs_action_resp"
                        assert data.get("success") is True
                        print("✅ fs_delete folder succeeded")
                        break

            print("\n🎉 ALL E2E FILE TRANSFER OPERATIONS PASSED SUCCESSFULLY!")

    finally:
        proc.terminate()
        try:
            proc.wait(timeout=2.0)
        except Exception:
            proc.kill()

if __name__ == "__main__":
    asyncio.run(main())
