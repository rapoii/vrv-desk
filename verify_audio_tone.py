import asyncio
import struct
import websockets
import time
import subprocess
import os

async def verify_audio_waveform():
    uri = "ws://127.0.0.1:53211"
    pin = "482910"
    print(f"Connecting to {uri}...")
    async with websockets.connect(uri) as ws:
        # 1. Wait for auth_required
        msg = await ws.recv()
        print(f"Server msg: {msg}")
        # 2. Send PIN
        await ws.send(f'{{"type":"auth_verify","pin":"{pin}"}}')
        # 3. Wait for auth_ok
        msg = await ws.recv()
        print(f"Server msg: {msg}")

        print("\n[*] Starting audio capture and tone playback on Windows...")
        # Play a continuous tone on Windows in background via powershell [Console]::Beep(1000, 3000)
        ps_cmd = 'powershell.exe -Command "[Console]::Beep(1000, 3000)"'
        proc = subprocess.Popen(ps_cmd, shell=True)

        non_zero_chunks = 0
        total_chunks = 0
        max_amplitude = 0
        start = time.time()

        while time.time() - start < 5.0:
            msg = await ws.recv()
            if isinstance(msg, bytes):
                if len(msg) >= 8 and msg[:4] == b"VAUD":
                    total_chunks += 1
                    payload = msg[8:]
                    # Parse as int16 little-endian
                    num_samples = len(payload) // 2
                    if num_samples > 0:
                        samples = struct.unpack(f"<{num_samples}h", payload)
                        current_max = max(abs(s) for s in samples)
                        if current_max > max_amplitude:
                            max_amplitude = current_max
                        if current_max > 50:  # Threshold for non-silence
                            non_zero_chunks += 1

        proc.terminate()
        print("\n" + "="*50)
        print(f"Total audio chunks received: {total_chunks}")
        print(f"Non-zero audio chunks detected: {non_zero_chunks}")
        print(f"Maximum amplitude recorded (int16 0-32767): {max_amplitude}")
        print("="*50)

        if max_amplitude > 100:
            print("🔊 REAL SOUND DETECTED IN STREAM! WASAPI loopback captured real audio waveform!")
        else:
            print("⚠️ Only silence detected. WASAPI loopback is streaming 0s.")

if __name__ == "__main__":
    asyncio.run(verify_audio_waveform())
