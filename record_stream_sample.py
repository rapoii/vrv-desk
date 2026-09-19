import asyncio
import struct
import wave
import time
import winsound
import threading
import websockets

async def record_audio_stream():
    uri = "ws://127.0.0.1:53211"
    pin = "482910"
    output_wav = "D:/Software/Hermes Workspace/projects/vrv-desk/captured_audio_sample.wav"
    
    print(f"Connecting to {uri}...")
    async with websockets.connect(uri) as ws:
        # 1. Handshake
        msg1 = await ws.recv()
        await ws.send(f'{{"type":"auth_verify","pin":"{pin}"}}')
        msg2 = await ws.recv()
        print("Authenticated successfully!")
        
        # 2. Start thread to play notes on Windows
        def play_sound():
            time.sleep(0.5)
            melody = [
                (523, 200), (587, 200), (659, 200), (698, 200),
                (784, 300), (880, 300), (988, 300), (1046, 500)
            ]
            for freq, dur in melody:
                try:
                    winsound.Beep(freq, dur)
                    time.sleep(0.05)
                except Exception as e:
                    print("Beep err:", e)
                    
        t = threading.Thread(target=play_sound, daemon=True)
        t.start()
        
        # 3. Collect audio packets for 4 seconds
        pcm_chunks = []
        sample_rate = 48000
        channels = 2
        
        start = time.time()
        while time.time() - start < 4.0:
            msg = await ws.recv()
            if isinstance(msg, bytes):
                if len(msg) >= 8 and msg[:4] == b"VAUD":
                    fmt = msg[4]
                    channels = msg[5]
                    sample_rate = struct.unpack("<H", msg[6:8])[0]
                    pcm = msg[8:]
                    pcm_chunks.append(pcm)
                    
        total_pcm = b"".join(pcm_chunks)
        print(f"Recorded {len(pcm_chunks)} audio packets ({len(total_pcm)} bytes)")
        
        with wave.open(output_wav, "wb") as wf:
            wf.setnchannels(channels)
            wf.setsampwidth(2) # 16-bit
            wf.setframerate(sample_rate)
            wf.writeframes(total_pcm)
            
        print(f"Saved WAV file to: {output_wav}")

if __name__ == "__main__":
    asyncio.run(record_audio_stream())
