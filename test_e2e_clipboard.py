import asyncio
import json
import websockets
import ctypes
from ctypes import wintypes

# Win32 API for clipboard check
user32 = ctypes.windll.user32
kernel32 = ctypes.windll.kernel32

CF_UNICODETEXT = 13
GMEM_MOVEABLE = 0x0002

user32.GetClipboardData.restype = ctypes.c_void_p
user32.GetClipboardData.argtypes = [ctypes.c_uint]
kernel32.GlobalLock.restype = ctypes.c_void_p
kernel32.GlobalLock.argtypes = [ctypes.c_void_p]
kernel32.GlobalUnlock.argtypes = [ctypes.c_void_p]
kernel32.GlobalAlloc.restype = ctypes.c_void_p
kernel32.GlobalAlloc.argtypes = [ctypes.c_uint, ctypes.c_size_t]
user32.SetClipboardData.restype = ctypes.c_void_p
user32.SetClipboardData.argtypes = [ctypes.c_uint, ctypes.c_void_p]

def get_windows_clipboard():
    if not user32.OpenClipboard(None):
        return None
    try:
        handle = user32.GetClipboardData(CF_UNICODETEXT)
        if not handle:
            return None
        ptr = kernel32.GlobalLock(handle)
        if not ptr:
            return None
        try:
            return ctypes.wstring_at(ptr)
        finally:
            kernel32.GlobalUnlock(handle)
    finally:
        user32.CloseClipboard()

def set_windows_clipboard(text: str):
    if not user32.OpenClipboard(None):
        return False
    try:
        user32.EmptyClipboard()
        bytes_data = (text + "\0").encode("utf-16le")
        h_mem = kernel32.GlobalAlloc(0x0002, len(bytes_data))
        if not h_mem:
            return False
        ptr = kernel32.GlobalLock(h_mem)
        if not ptr:
            kernel32.GlobalFree(h_mem)
            return False
        ctypes.memmove(ptr, bytes_data, len(bytes_data))
        kernel32.GlobalUnlock(h_mem)
        user32.SetClipboardData(CF_UNICODETEXT, h_mem)
        return True
    finally:
        user32.CloseClipboard()

async def main():
    uri = "ws://127.0.0.1:53211"
    print(f"Connecting to {uri}...")
    async with websockets.connect(uri) as ws:
        print("Connected!")

        # 1. Test video frame reception
        frame = await ws.recv()
        assert isinstance(frame, bytes) and len(frame) > 1000
        print(f"Received video frame: {len(frame)} bytes")

        # 2. Test typing text
        print("Testing type_text...")
        await ws.send(json.dumps({
            "type": "type_text",
            "text": "Hello VrV"
        }))

        # 3. Test shortcuts (win, esc)
        print("Testing shortcuts...")
        await ws.send(json.dumps({
            "type": "shortcut",
            "name": "win"
        }))
        await asyncio.sleep(0.3)
        await ws.send(json.dumps({
            "type": "shortcut",
            "name": "esc"
        }))

        # 4. Test Android -> PC clipboard
        test_phone_text = "VrV-Desk-Phone-Clipboard-Unique-999"
        print(f"Sending clipboard_text: {test_phone_text}...")
        await ws.send(json.dumps({
            "type": "clipboard_text",
            "text": test_phone_text
        }))

        # Wait for host to update clipboard
        await asyncio.sleep(0.5)
        pc_clip = get_windows_clipboard()
        print(f"Windows Clipboard is now: {pc_clip}")
        assert pc_clip == test_phone_text, f"Expected '{test_phone_text}', got '{pc_clip}'"
        print("Android -> PC Clipboard Sync Verified!")

        # 5. Test PC -> Android clipboard sync broadcast
        test_pc_text = "VrV-Desk-PC-Clipboard-Broadcast-888"
        print(f"Setting Windows clipboard to: {test_pc_text}...")
        set_windows_clipboard(test_pc_text)

        print("Waiting for clipboard_sync WebSocket broadcast from PC host...")
        received_sync = False
        for _ in range(50):
            try:
                msg = await asyncio.wait_for(ws.recv(), timeout=2.0)
                if isinstance(msg, str):
                    data = json.loads(msg)
                    print(f"Received text message: {data}")
                    if data.get("type") == "clipboard_sync" and data.get("text") == test_pc_text:
                        print(f"Received expected broadcast: {data}")
                        received_sync = True
                        break
            except asyncio.TimeoutError:
                continue

        assert received_sync, "Did not receive clipboard_sync message from host"
        print("PC -> Android Clipboard Broadcast Verified!")
        print("ALL E2E CLIPBOARD & SHORTCUT TESTS PASSED!")

if __name__ == "__main__":
    asyncio.run(main())
