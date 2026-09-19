import socket
import struct
import json
import subprocess
import time
import sys
import os

MULTICAST_GROUP = "239.255.42.99"
DISCOVERY_PORT = 53210

def test_lan_discovery_e2e():
    print(f"=== E2E Test: LAN UDP Discovery Beacon (Port {DISCOVERY_PORT}, Group {MULTICAST_GROUP}) ===")
    
    # 1. Setup UDP listener socket
    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM, socket.IPPROTO_UDP)
    sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    
    # Bind to wildcard address on DISCOVERY_PORT
    try:
        sock.bind(("", DISCOVERY_PORT))
    except Exception as e:
        print(f"Failed to bind UDP port {DISCOVERY_PORT}: {e}")
        return False

    # Join multicast group
    mreq = struct.pack("4sl", socket.inet_aton(MULTICAST_GROUP), socket.INADDR_ANY)
    try:
        sock.setsockopt(socket.IPPROTO_IP, socket.IP_ADD_MEMBERSHIP, mreq)
        print(f"Joined multicast group {MULTICAST_GROUP}")
    except Exception as e:
        print(f"Warning: Could not join multicast group ({e}), falling back to broadcast/unicast")

    sock.settimeout(5.0)

    # 2. Launch vrv_host.exe
    host_exe = os.path.abspath("D:/Software/Hermes Workspace/projects/vrv-desk/dist/windows/vrv_host.exe")
    print(f"Starting host: {host_exe} --pin 482910")
    
    proc = subprocess.Popen([host_exe, "--pin", "482910"], stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    time.sleep(1.0)

    try:
        print("Waiting for LAN discovery beacon...")
        start_time = time.time()
        received = False

        while time.time() - start_time < 6.0:
            try:
                data, addr = sock.recvfrom(2048)
                print(f"Received {len(data)} bytes from {addr}")
                
                payload = json.loads(data.decode("utf-8"))
                print(f"Beacon payload decoded: {payload}")

                # Validate fields
                assert "device_id" in payload, "Missing device_id"
                assert "device_name" in payload, "Missing device_name"
                assert payload.get("os_type") == "windows", f"Expected os_type=windows, got {payload.get('os_type')}"
                assert payload.get("port") == 53211, f"Expected port=53211, got {payload.get('port')}"
                assert payload.get("protocol_version") == 1, f"Expected protocol_version=1, got {payload.get('protocol_version')}"

                print("✅ LAN Discovery Beacon successfully validated!")
                print(f"   Device ID: {payload['device_id']}")
                print(f"   Device Name: {payload['device_name']}")
                print(f"   OS: {payload['os_type']}")
                print(f"   Service Port: {payload['port']}")
                received = True
                break
            except socket.timeout:
                print("Socket timeout tick, continuing wait...")
            except Exception as e:
                print(f"Error parsing packet: {e}")

        if not received:
            print("❌ Failed to receive valid LAN beacon within timeout period!")
            return False

        return True

    finally:
        sock.close()
        proc.terminate()
        try:
            proc.wait(timeout=2.0)
        except Exception:
            proc.kill()
        print("Cleaned up host process and socket.")

if __name__ == "__main__":
    success = test_lan_discovery_e2e()
    sys.exit(0 if success else 1)
