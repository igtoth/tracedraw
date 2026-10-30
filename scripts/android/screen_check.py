"""Check that a screenshot of the Android app shows a drawn interface.

Usage: python3 screen_check.py screen.png

`adb exec-out screencap -p` writes an 8-bit RGBA (or RGB) PNG. A window
that never drew is one flat colour (black, or the theme's background);
the editor shows bars, icons, the page and text in many colours. The
check passes when no single colour covers more than 97 % of the screen
and the screen has at least 16 different colours. Standard library only,
so it runs on any CI runner. Exit code 0: drawn; 1: blank; 2: not a PNG
it can read.
"""

import struct
import sys
import zlib

MAX_SHARE = 0.97
MIN_COLOURS = 16


def read_png(path):
    """Width, height, channels and the unfiltered rows of an 8-bit PNG."""
    with open(path, "rb") as f:
        data = f.read()
    if data[:8] != b"\x89PNG\r\n\x1a\n":
        raise ValueError("not a PNG file")
    pos = 8
    idat = []
    width = height = channels = None
    while pos + 8 <= len(data):
        (length,) = struct.unpack(">I", data[pos : pos + 4])
        kind = data[pos + 4 : pos + 8]
        body = data[pos + 8 : pos + 8 + length]
        pos += 12 + length
        if kind == b"IHDR":
            width, height, depth, colour, _, _, interlace = struct.unpack(">IIBBBBB", body)
            if depth != 8 or interlace != 0 or colour not in (2, 6):
                raise ValueError(f"unsupported PNG (depth {depth}, colour type {colour}, interlace {interlace})")
            channels = 3 if colour == 2 else 4
        elif kind == b"IDAT":
            idat.append(body)
        elif kind == b"IEND":
            break
    if width is None or not idat:
        raise ValueError("no image data")
    raw = zlib.decompress(b"".join(idat))
    stride = width * channels
    if len(raw) < height * (stride + 1):
        raise ValueError("image data too short")
    rows = []
    prev = bytearray(stride)
    for y in range(height):
        start = y * (stride + 1)
        kind = raw[start]
        line = bytearray(raw[start + 1 : start + 1 + stride])
        if kind == 1:
            for i in range(channels, stride):
                line[i] = (line[i] + line[i - channels]) & 0xFF
        elif kind == 2:
            for i in range(stride):
                line[i] = (line[i] + prev[i]) & 0xFF
        elif kind == 3:
            for i in range(stride):
                left = line[i - channels] if i >= channels else 0
                line[i] = (line[i] + ((left + prev[i]) >> 1)) & 0xFF
        elif kind == 4:
            for i in range(stride):
                a = line[i - channels] if i >= channels else 0
                b = prev[i]
                c = prev[i - channels] if i >= channels else 0
                p = a + b - c
                pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
                if pa <= pb and pa <= pc:
                    pred = a
                elif pb <= pc:
                    pred = b
                else:
                    pred = c
                line[i] = (line[i] + pred) & 0xFF
        elif kind != 0:
            raise ValueError(f"unknown filter {kind} in row {y}")
        rows.append(line)
        prev = line
    return width, height, channels, rows


def colour_counts(width, channels, rows):
    """How many pixels have each RGB colour."""
    counts = {}
    for line in rows:
        for x in range(width):
            i = x * channels
            key = (line[i], line[i + 1], line[i + 2])
            counts[key] = counts.get(key, 0) + 1
    return counts


def main(argv):
    if len(argv) != 2:
        print(__doc__.strip().splitlines()[2], file=sys.stderr)
        return 2
    try:
        width, height, channels, rows = read_png(argv[1])
    except (OSError, ValueError, zlib.error) as e:
        print(f"screen_check: {argv[1]}: {e}", file=sys.stderr)
        return 2
    counts = colour_counts(width, channels, rows)
    total = width * height
    top, top_count = max(counts.items(), key=lambda kv: kv[1])
    share = top_count / total
    print(
        f"screen {width}x{height}: {len(counts)} colours, the most common "
        f"#{top[0]:02x}{top[1]:02x}{top[2]:02x} on {share:.1%} of it"
    )
    if share > MAX_SHARE or len(counts) < MIN_COLOURS:
        print("screen_check: the screen is blank: nothing was drawn", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
