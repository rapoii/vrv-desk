import asyncio
import json
import os
import subprocess
import sys
import time
import websockets

async def main():
    uri = "ws://127.0.0.1:53211"
    print("Testing VrV Desk Unattended Access E2E...")

    # 1. Test Unattended Access with Permanent Password
    print("\n--- Test 1: Connect with Unattended Permanent Password ---")
    async with websockets.connect(uri) as ws:
        init_msg = await ws.recv()
        init_json = json.loads(init_msg)
        assert init_json["type"] == "auth_required", f"Expected auth_required, got {init_json}"

        # Send auth_verify using permanent unattended password
        print("Sending auth_verify with unattended password...")
        await ws.send(json.dumps({
            "type": "auth_verify",
            "pin": "PermanentSecret2026",
            "e2ee": False
        }))

        auth_resp_raw = await ws.recv()
        auth_resp = json.loads(auth_resp_raw)
        print(f"Auth response: {auth_resp}")
        assert auth_resp["type"] == "auth_ok", f"Expected auth_ok, got {auth_resp}"
        assert auth_resp.get("unattended") is True, f"Expected unattended=true, got {auth_resp}"

        # Verify initial packet arrives
        packet = await ws.recv()
        assert isinstance(packet, (bytes, str)), "Stream active"
        print("✅ Client 1 successfully authenticated via Unattended Access!")

    # 2. Test Dynamic PIN still works in parallel
    print("\n--- Test 2: Connect with Dynamic PIN ---")
    async with websockets.connect(uri) as ws:
        init_msg = await ws.recv()
        init_json = json.loads(init_msg)
        assert init_json["type"] == "auth_required"

        print("Sending auth_verify with 6-digit dynamic PIN...")
        await ws.send(json.dumps({
            "type": "auth_verify",
            "pin": "112233",
            "e2ee": False
        }))

        auth_resp_raw = await ws.recv()
        auth_resp = json.loads(auth_resp_raw)
        print(f"Auth response: {auth_resp}")
        assert auth_resp["type"] == "auth_ok"
        assert auth_resp.get("unattended") is False
        print("✅ Client 2 successfully authenticated via Dynamic PIN!")

    # 3. Test Invalid Password Rejection
    print("\n--- Test 3: Reject Wrong Password ---")
    async with websockets.connect(uri) as ws:
        init_msg = await ws.recv()
        init_json = json.loads(init_msg)
        assert init_json["type"] == "auth_required"

        print("Sending auth_verify with incorrect password...")
        await ws.send(json.dumps({
            "type": "auth_verify",
            "pin": "WrongPassword999",
            "e2ee": False
        }))

        auth_resp_raw = await ws.recv()
        auth_resp = json.loads(auth_resp_raw)
        print(f"Auth response: {auth_resp}")
        assert auth_resp["type"] == "auth_failed"
        print("✅ Client 3 rejected as expected!")

    print("\n🎉 ALL E2E UNATTENDED ACCESS TESTS PASSED SUCCESSFULLY!")

if __name__ == "__main__":
    asyncio.run(main())
