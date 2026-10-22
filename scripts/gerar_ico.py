"""Monta assets/app/fenix.ico a partir de PNGs (um por tamanho), com entradas PNG.

    python scripts/gerar_ico.py saida.ico icone-256.png icone-48.png icone-32.png icone-16.png
"""
import struct
import sys


def png_size(data):
    w, h = struct.unpack(">II", data[16:24])
    return w, h


out, pngs = sys.argv[1], sys.argv[2:]
images = [open(p, "rb").read() for p in pngs]
header = struct.pack("<HHH", 0, 1, len(images))
offset = 6 + 16 * len(images)
entries, blobs = b"", b""
for data in images:
    w, h = png_size(data)
    entries += struct.pack("<BBBBHHII", w % 256, h % 256, 0, 0, 1, 32, len(data), offset)
    blobs += data
    offset += len(data)
open(out, "wb").write(header + entries + blobs)
print("ok", out, [png_size(d) for d in images])
