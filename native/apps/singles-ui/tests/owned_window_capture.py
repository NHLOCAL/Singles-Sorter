"""Bounded by the parent process: render one PID-verified HWND, never the desktop."""
import os
from pathlib import Path
import struct
import sys
import zlib


def png_from_bgra(width, height, pixels):
    def chunk(kind, data):
        return struct.pack('!I', len(data)) + kind + data + struct.pack('!I', zlib.crc32(kind + data))
    rows = []
    for y in range(height):
        row = pixels[y * width * 4:(y + 1) * width * 4]
        rgb = bytearray(width * 3)
        rgb[0::3], rgb[1::3], rgb[2::3] = row[2::4], row[1::4], row[0::4]
        rows.append(b'\0' + rgb)
    return (b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('!IIBBBBB', width, height, 8, 2, 0, 0, 0))
            + chunk(b'IDAT', zlib.compress(b''.join(rows))) + chunk(b'IEND', b''))


def capture(hwnd, pid, destination):
    import ctypes
    from ctypes import wintypes
    import win32gui
    import win32con
    import win32ui
    import win32process

    if win32process.GetWindowThreadProcessId(hwnd)[1] != pid:
        raise RuntimeError('Window no longer belongs to the owned process')
    left, top, right, bottom = win32gui.GetWindowRect(hwnd)
    width, height = right - left, bottom - top
    if not (0 < width <= 4096 and 0 < height <= 4096):
        raise RuntimeError('Window dimensions are outside diagnostic bounds')
    dc = win32gui.GetWindowDC(hwnd)
    source = win32ui.CreateDCFromHandle(dc)
    target = source.CreateCompatibleDC()
    bitmap = win32ui.CreateBitmap()
    bitmap.CreateCompatibleBitmap(source, width, height)
    old = target.SelectObject(bitmap)
    try:
        target.PatBlt((0, 0), (width, height), win32con.BLACKNESS)
        print_window = ctypes.WinDLL('user32', use_last_error=True).PrintWindow
        print_window.argtypes = [wintypes.HWND, wintypes.HDC, wintypes.UINT]
        print_window.restype = wintypes.BOOL
        # PW_RENDERFULLCONTENT. No BitBlt, screen grab, or fallback on failure.
        if not print_window(hwnd, target.GetSafeHdc(), 2):
            raise RuntimeError('PrintWindow could not render the owned window')
        if win32process.GetWindowThreadProcessId(hwnd)[1] != pid:
            raise RuntimeError('Window ownership changed during capture')
        pixels = bitmap.GetBitmapBits(True)
        if len(pixels) != width * height * 4:
            raise RuntimeError('Unexpected bitmap format')
        destination.write_bytes(png_from_bgra(width, height, pixels))
    finally:
        target.SelectObject(old)
        win32gui.DeleteObject(bitmap.GetHandle())
        target.DeleteDC()
        source.DeleteDC()
        win32gui.ReleaseDC(hwnd, dc)


if __name__ == '__main__':
    if os.name != 'nt' or os.environ.get('GITHUB_ACTIONS') != 'true':
        raise SystemExit('Owned-window capture is restricted to Windows CI')
    capture(int(sys.argv[1]), int(sys.argv[2]), Path(sys.argv[3]))
