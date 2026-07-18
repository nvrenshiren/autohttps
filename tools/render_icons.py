"""autohttps 图标渲染器 —— 纯 Python(无 Pillow/ImageMagick 依赖),确定性重生成全部图标。

用法: python tools/render_icons.py
产物: crates/desktop/icons/{icon.png, 128x128.png, 128x128@2x.png, 64x64.png, 48x48.png,
      32x32.png, 24x24.png, 16x16.png, icon.ico}
设计源(可读): assets/icon.svg(主几何;本脚本才是构建用真相,SVG 仅展示/文档用)。

两套几何:主几何(1024..48)与加粗变体(32/24/16 —— 锁体放大 ~1.2x、描边加粗,
保证托盘小尺寸下轮廓可辨)。渲染 = 解析式 SDF + 预乘 alpha 超采样抗锯齿。
"""
import math, struct, zlib

OUT_DIR = "crates/desktop/icons"

BG_TOP = (0x35, 0x43, 0xD6)   # 群青渐变顶(Blueprint primary oklch(0.5 0.2 268) 近似)
BG_BOT = (0x1F, 0x29, 0xA0)   # 群青渐变底
PAPER  = (0xF7, 0xF3, 0xEA)   # 暖纸白(主题 background oklch(0.982 0.004 90) 近似)
INK    = (0x22, 0x30, 0xB8)   # 对勾用深群青

def make_shade(W, H, squircle_r, stroke, key, check_w, body, shackle):
    def lerp(a, b, t): return a + (b - a) * t
    def sqdist_seg(px, py, x1, y1, x2, y2):
        dx, dy = x2 - x1, y2 - y1
        if dx == 0 and dy == 0: return (px - x1) ** 2 + (py - y1) ** 2
        t = max(0, min(1, ((px - x1) * dx + (py - y1) * dy) / (dx * dx + dy * dy)))
        cx, cy = x1 + t * dx, y1 + t * dy
        return (px - cx) ** 2 + (py - cy) ** 2
    def inside_squircle(x, y):
        r = squircle_r
        cx = min(max(x, r), W - r); cy = min(max(y, r), H - r)
        return (x - cx) ** 2 + (y - cy) ** 2 <= r * r
    (sx0, _, sx1, sy1, arch_cx, arch_cy, arch_r) = shackle
    def shackle_dist(x, y):
        d1 = sqdist_seg(x, y, sx0, arch_cy, sx0, sy1) ** 0.5
        d2 = sqdist_seg(x, y, sx1, arch_cy, sx1, sy1) ** 0.5
        dd = math.hypot(x - arch_cx, y - arch_cy)
        darch = abs(dd - arch_r) if y <= arch_cy + 8 else 1e9
        return min(d1, d2, darch)
    (bx, by, bw, bh, br) = body
    def body_ring(x, y):
        def inside(rx0, ry0, rw, rh, rr):
            cx = min(max(x, rx0 + rr), rx0 + rw - rr); cy = min(max(y, ry0 + rr), ry0 + rh - rr)
            if rx0 + rr <= x <= rx0 + rw - rr and ry0 <= y <= ry0 + rh: return True
            if rx0 <= x <= rx0 + rw and ry0 + rr <= y <= ry0 + rh - rr: return True
            return (x - cx) ** 2 + (y - cy) ** 2 <= rr * rr
        s = stroke
        return inside(bx, by, bw, bh, br) and not inside(bx + s, by + s, bw - 2 * s, bh - 2 * s, max(1, br - s))
    (kr, ky) = key
    kx = body[0] + body[2] // 2
    CHECK = [(kx - 0.69 * kr, ky - 0.16 * kr), (kx - 0.20 * kr, ky + 0.31 * kr), (kx + 0.69 * kr, ky - 0.58 * kr)]
    def check_dist(x, y):
        return min(sqdist_seg(x, y, *CHECK[0], *CHECK[1]), sqdist_seg(x, y, *CHECK[1], *CHECK[2])) ** 0.5
    def shade(x, y):
        if not inside_squircle(x, y): return (0, 0, 0, 0)
        t = y / H
        base = tuple(round(lerp(BG_TOP[i], BG_BOT[i], t)) for i in range(3))
        hl = 0.06 * 255 if y < 0.41 * H else 0
        col = (min(255, base[0] + round(hl)), min(255, base[1] + round(hl)), min(255, base[2] + round(hl)), 255)
        if shackle_dist(x, y) <= stroke / 2 or body_ring(x, y): col = (*PAPER, 255)
        if (x - kx) ** 2 + (y - ky) ** 2 <= kr * kr:
            col = (*PAPER, 255)
            if check_dist(x, y) <= check_w / 2: col = (*INK, 255)
        return col
    return shade

def render_png(path, out_size, shade, src, AA):
    scale = src / out_size
    raw = bytearray()
    for yy in range(out_size):
        raw.append(0)
        for xx in range(out_size):
            acc = [0, 0, 0, 0]; n = 0
            for sy in range(AA):
                for sx in range(AA):
                    fx = (xx + (sx + 0.5) / AA) * scale; fy = (yy + (sy + 0.5) / AA) * scale
                    c = shade(fx, fy); a = c[3] / 255
                    acc[0] += c[0] * a; acc[1] += c[1] * a; acc[2] += c[2] * a; acc[3] += c[3]; n += 1
            A = acc[3] / n
            r = g = b = 0
            if A > 0:
                r = round(acc[0] / n * 255 / A); g = round(acc[1] / n * 255 / A); b = round(acc[2] / n * 255 / A)
            raw.extend((r, g, b, round(A)))
    def chunk(tag, data):
        c = tag + data
        return struct.pack(">I", len(data)) + c + struct.pack(">I", zlib.crc32(c) & 0xffffffff)
    png = (b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", out_size, out_size, 8, 6, 0, 0, 0))
           + chunk(b"IDAT", zlib.compress(bytes(raw), 9)) + chunk(b"IEND", b""))
    open(path, "wb").write(png)

def pack_ico(path, frames):
    header = struct.pack("<HHH", 0, 1, len(frames))
    offset = 6 + 16 * len(frames)
    entries = b""; blob = b""
    for s, data in frames:
        w = 0 if s >= 256 else s
        entries += struct.pack("<BBBBHHII", w, w, 0, 0, 1, 32, len(data), offset)
        blob += data; offset += len(data)
    open(path, "wb").write(header + entries + blob)

def main():
    # 主几何(1024 基准)
    main_shade = make_shade(1024, 1024, 236, 76, (74, 628), 38,
                            body=(256, 470, 512, 400, 84),
                            shackle=(352, 0, 672, 470, 512, 372, 160))
    # 小尺寸加粗变体:锁体放大 ~1.2x、描边 +16、圆章/对勾加大
    small_shade = make_shade(1024, 1024, 210, 92, (92, 640), 46,
                             body=(212, 436, 600, 460, 96),
                             shackle=(317, 0, 707, 436, 512, 342, 195))
    jobs = [  # (文件名, 尺寸, 几何, 超采样)
        ("icon.png", 1024, main_shade, 1),
        ("128x128@2x.png", 256, main_shade, 4),
        ("128x128.png", 128, main_shade, 8),
        ("64x64.png", 64, main_shade, 16),
        ("48x48.png", 48, main_shade, 21),
        ("32x32.png", 32, small_shade, 32),
        ("24x24.png", 24, small_shade, 42),
        ("16x16.png", 16, small_shade, 64),
    ]
    for name, size, shade, aa in jobs:
        render_png(f"{OUT_DIR}/{name}", size, shade, 1024, aa)
        print(f"  {name} ({size}x{size}, AA x{aa})")
    pack_ico(f"{OUT_DIR}/icon.ico", [
        (s, open(f"{OUT_DIR}/{n}", "rb").read())
        for s, n in [(16, "16x16.png"), (24, "24x24.png"), (32, "32x32.png"), (48, "48x48.png"),
                     (64, "64x64.png"), (128, "128x128.png"), (256, "128x128@2x.png")]
    ])
    print("  icon.ico (16/24/32/48/64/128/256)")

if __name__ == "__main__":
    main()
