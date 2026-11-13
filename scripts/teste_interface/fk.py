"""Equipamento de teste da interface do Fenkra com mouse e teclado reais (B41).

Só biblioteca padrão e ctypes. Usa SendInput do Windows, então o app recebe os mesmos
eventos que receberia de uma pessoa: nada de variáveis FENKRA_DEMO nem atalhos internos.

Uso pela linha de comando (cada comando termina com uma captura):

    python -I fk.py abrir --exe <fenkra.exe> --appdata <pasta> [--saida <pasta>]
    python -I fk.py foto <nome>                       # só captura
    python -I fk.py clicar <x> <y> [--botao dir|meio] [--duplo]
    python -I fk.py mover <x> <y> [--esperar 1.5]     # passar o mouse (tooltip)
    python -I fk.py arrastar <x0> <y0> <x1> <y1> [--passos 20] [--botao meio]
    python -I fk.py rolar <x> <y> <ticks>             # positivo = para cima
    python -I fk.py tecla ctrl+z | f2 | tab | shift+tab | alt+up | pgdn ...
    python -I fk.py digitar "texto"
    python -I fk.py roteiro <arquivo.json>
    python -I fk.py recorte <png> <x0> <y0> <x1> <y1> <saida.png> [--escala 2]
    python -I fk.py reduzir <png> <saida.png> [--fator 2]
    python -I fk.py fechar

Coordenadas são sempre relativas à área cliente da janela, em pixels físicos, as mesmas da
captura. Antes de cada ação o equipamento confere que o Fenkra está na frente; se não estiver,
espera 30 s e tenta de novo; com três falhas seguidas, encerra com código 3. Nenhum clique sai
da janela: ponto fora da área cliente é recusado.

Roteiro JSON: {"passos": [{"acao": "clicar", "x": 100, "y": 200, "nome": "abre_menu"}, ...]}.
Ações do roteiro: clicar, duplo, direito, mover, arrastar, rolar, tecla, digitar, esperar, foto.
"""

from __future__ import annotations

import argparse
import ctypes
import json
import os
import struct
import subprocess
import sys
import time
import zlib
from ctypes import wintypes as W
from pathlib import Path

user32 = ctypes.WinDLL("user32", use_last_error=True)
gdi32 = ctypes.WinDLL("gdi32", use_last_error=True)
kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)

try:  # pixels físicos, para bater com a captura
    user32.SetProcessDpiAwarenessContext(ctypes.c_void_p(-4))
except Exception:
    pass

ESTADO = Path(os.environ.get("FK_ESTADO", Path(os.environ.get("TEMP", ".")) / "fk_estado.json"))

# ---------------------------------------------------------------- estruturas do SendInput
ULONG_PTR = ctypes.c_size_t


class MOUSEINPUT(ctypes.Structure):
    _fields_ = [("dx", W.LONG), ("dy", W.LONG), ("mouseData", W.DWORD),
                ("dwFlags", W.DWORD), ("time", W.DWORD), ("dwExtraInfo", ULONG_PTR)]


class KEYBDINPUT(ctypes.Structure):
    _fields_ = [("wVk", W.WORD), ("wScan", W.WORD), ("dwFlags", W.DWORD),
                ("time", W.DWORD), ("dwExtraInfo", ULONG_PTR)]


class HARDWAREINPUT(ctypes.Structure):
    _fields_ = [("uMsg", W.DWORD), ("wParamL", W.WORD), ("wParamH", W.WORD)]


class _U(ctypes.Union):
    _fields_ = [("mi", MOUSEINPUT), ("ki", KEYBDINPUT), ("hi", HARDWAREINPUT)]


class INPUT(ctypes.Structure):
    _fields_ = [("type", W.DWORD), ("u", _U)]


INPUT_MOUSE, INPUT_KEYBOARD = 0, 1
MOVE, ABS, VDESK = 0x0001, 0x8000, 0x4000
LDOWN, LUP, RDOWN, RUP, MDOWN, MUP, WHEEL = 0x2, 0x4, 0x8, 0x10, 0x20, 0x40, 0x800
K_EXT, K_UP, K_UNI, K_SCAN = 0x1, 0x2, 0x4, 0x8

user32.SendInput.argtypes = [W.UINT, ctypes.POINTER(INPUT), ctypes.c_int]
user32.GetForegroundWindow.restype = W.HWND
user32.FindWindowW.restype = W.HWND
user32.WindowFromPoint.restype = W.HWND
user32.WindowFromPoint.argtypes = [W.POINT]
user32.GetAncestor.restype = W.HWND
user32.GetAncestor.argtypes = [W.HWND, W.UINT]
user32.FindWindowW.argtypes = [W.LPCWSTR, W.LPCWSTR]
user32.GetDC.restype = W.HDC
user32.GetDC.argtypes = [W.HWND]
user32.ReleaseDC.argtypes = [W.HWND, W.HDC]
user32.PrintWindow.argtypes = [W.HWND, W.HDC, W.UINT]
gdi32.CreateCompatibleDC.restype = W.HDC
gdi32.CreateCompatibleDC.argtypes = [W.HDC]
gdi32.CreateCompatibleBitmap.restype = W.HBITMAP
gdi32.CreateCompatibleBitmap.argtypes = [W.HDC, ctypes.c_int, ctypes.c_int]
gdi32.SelectObject.restype = W.HGDIOBJ
gdi32.SelectObject.argtypes = [W.HDC, W.HGDIOBJ]
gdi32.BitBlt.argtypes = [W.HDC, ctypes.c_int, ctypes.c_int, ctypes.c_int, ctypes.c_int,
                         W.HDC, ctypes.c_int, ctypes.c_int, W.DWORD]
gdi32.DeleteObject.argtypes = [W.HGDIOBJ]
gdi32.DeleteDC.argtypes = [W.HDC]
gdi32.GetDIBits.argtypes = [W.HDC, W.HBITMAP, W.UINT, W.UINT, ctypes.c_void_p,
                            ctypes.c_void_p, W.UINT]

VK = {
    "ctrl": 0x11, "shift": 0x10, "alt": 0x12, "win": 0x5B, "enter": 0x0D, "esc": 0x1B,
    "tab": 0x09, "space": 0x20, "back": 0x08, "del": 0x2E, "delete": 0x2E, "ins": 0x2D,
    "home": 0x24, "end": 0x23, "pgup": 0x21, "pgdn": 0x22, "left": 0x25, "up": 0x26,
    "right": 0x27, "down": 0x28,
}
VK.update({f"f{i}": 0x6F + i for i in range(1, 13)})
EXTENDIDAS = {0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x28, 0x2D, 0x2E, 0x5B}


class SemFoco(RuntimeError):
    pass


# ---------------------------------------------------------------- janela
def _estado() -> dict:
    try:
        return json.loads(ESTADO.read_text(encoding="utf-8"))
    except Exception:
        return {}


def _gravar_estado(d: dict) -> None:
    ESTADO.write_text(json.dumps(d, indent=1), encoding="utf-8")


def janelas_do_processo(pid: int) -> list[int]:
    achadas: list[int] = []
    PROC = ctypes.WINFUNCTYPE(W.BOOL, W.HWND, W.LPARAM)

    def cb(h, _):
        p = W.DWORD()
        user32.GetWindowThreadProcessId(h, ctypes.byref(p))
        if p.value == pid and user32.IsWindowVisible(h) and not user32.GetWindow(h, 4):
            t = ctypes.create_unicode_buffer(256)
            user32.GetWindowTextW(h, t, 256)
            if t.value:  # a janela de eventos do winit não tem título
                achadas.append(h)
        return True

    user32.EnumWindows(PROC(cb), 0)
    return achadas


def hwnd() -> int:
    st = _estado()
    h = st.get("hwnd")
    if h and user32.IsWindow(h):
        return h
    pid = st.get("pid")
    if pid:
        hs = janelas_do_processo(pid)
        if hs:
            st["hwnd"] = hs[0]
            _gravar_estado(st)
            return hs[0]
    h = user32.FindWindowW(None, "Fenkra")
    if not h:
        raise SystemExit("janela do Fenkra não encontrada")
    return h


def cliente(h: int) -> tuple[int, int, int, int]:
    """Origem da área cliente na tela e seu tamanho."""
    r = W.RECT()
    user32.GetClientRect(h, ctypes.byref(r))
    p = W.POINT(0, 0)
    user32.ClientToScreen(h, ctypes.byref(p))
    return p.x, p.y, r.right - r.left, r.bottom - r.top


def na_frente(h: int) -> bool:
    return user32.GetForegroundWindow() == h


def trazer(h: int) -> bool:
    if user32.IsIconic(h):
        user32.ShowWindow(h, 9)  # SW_RESTORE
    # truque conhecido: um Alt solto destrava SetForegroundWindow vindo de outro processo
    _tecla_vk(VK["alt"], True)
    _tecla_vk(VK["alt"], False)
    user32.SetForegroundWindow(h)
    time.sleep(0.3)
    return na_frente(h)


def garantir_foco(h: int, espera: float = 30.0, tentativas: int = 3) -> None:
    """Confere o foco antes de cada ação. Se perdeu, espera e tenta de novo."""
    if na_frente(h):
        return
    for i in range(tentativas):
        print(f"[foco] Fenkra fora da frente; esperando {espera:.0f} s (tentativa {i + 1})",
              file=sys.stderr)
        time.sleep(espera)
        if trazer(h):
            return
    raise SemFoco("o Fenkra perdeu o foco três vezes seguidas")


def maximizar(h: int) -> None:
    user32.ShowWindow(h, 3)  # SW_MAXIMIZE
    time.sleep(0.8)


# ---------------------------------------------------------------- entrada
def _enviar(*ins: INPUT) -> None:
    arr = (INPUT * len(ins))(*ins)
    n = user32.SendInput(len(ins), arr, ctypes.sizeof(INPUT))
    if n != len(ins):
        raise OSError(f"SendInput recusou ({ctypes.get_last_error()})")


def _abs(x: int, y: int) -> tuple[int, int]:
    vx, vy = user32.GetSystemMetrics(76), user32.GetSystemMetrics(77)
    vw, vh = user32.GetSystemMetrics(78), user32.GetSystemMetrics(79)
    return (round((x - vx) * 65535 / (vw - 1)), round((y - vy) * 65535 / (vh - 1)))


def _mouse(flags: int, x: int = 0, y: int = 0, dado: int = 0) -> INPUT:
    i = INPUT(type=INPUT_MOUSE)
    i.u.mi = MOUSEINPUT(x, y, ctypes.c_uint32(dado).value, flags, 0, 0)
    return i


def _tecla_vk(vk: int, baixo: bool) -> None:
    scan = user32.MapVirtualKeyW(vk, 0)
    f = (0 if baixo else K_UP) | (K_EXT if vk in EXTENDIDAS else 0)
    i = INPUT(type=INPUT_KEYBOARD)
    i.u.ki = KEYBDINPUT(vk, scan, f, 0, 0)
    _enviar(i)


class Fenkra:
    """Sessão sobre a janela já aberta."""

    def __init__(self, saida: Path | None = None, espera_foco: float = 30.0):
        self.h = hwnd()
        st = _estado()
        self.saida = Path(saida or st.get("saida") or ".")
        self.saida.mkdir(parents=True, exist_ok=True)
        self.espera_foco = espera_foco
        self.passo = int(st.get("passo", 0))

    # coordenada da área cliente para a tela, recusando o que estiver fora
    def tela(self, x: int, y: int) -> tuple[int, int]:
        ox, oy, w, hh = cliente(self.h)
        if not (0 <= x < w and 0 <= y < hh):
            raise ValueError(f"ponto ({x}, {y}) fora da janela ({w}x{hh}); clique recusado")
        sx, sy = ox + x, oy + y
        alvo = user32.WindowFromPoint(W.POINT(sx, sy))
        if alvo and user32.GetAncestor(alvo, 2) != self.h:  # GA_ROOT
            raise ValueError(f"ponto ({x}, {y}) coberto por outra janela; clique recusado")
        return sx, sy

    def _ir(self, x: int, y: int) -> None:
        sx, sy = self.tela(x, y)
        ax, ay = _abs(sx, sy)
        _enviar(_mouse(MOVE | ABS | VDESK, ax, ay))

    def foco(self) -> None:
        garantir_foco(self.h, self.espera_foco)

    def mover(self, x: int, y: int, passos: int = 8) -> None:
        self.foco()
        p = W.POINT()
        user32.GetCursorPos(ctypes.byref(p))
        ox, oy, _, _ = cliente(self.h)
        x0, y0 = p.x - ox, p.y - oy
        dentro = 0 <= x0 and 0 <= y0
        for k in range(1, passos + 1):
            t = k / passos
            if dentro:
                self._ir(round(x0 + (x - x0) * t), round(y0 + (y - y0) * t))
            time.sleep(0.012)
        self._ir(x, y)
        time.sleep(0.05)

    def clicar(self, x: int, y: int, botao: str = "esq", duplo: bool = False,
               mods: tuple[str, ...] = ()) -> None:
        self.mover(x, y)
        d, u = {"esq": (LDOWN, LUP), "dir": (RDOWN, RUP), "meio": (MDOWN, MUP)}[botao]
        for m in mods:
            _tecla_vk(VK[m], True)
            time.sleep(0.03)
        for _ in range(2 if duplo else 1):
            _enviar(_mouse(d))
            time.sleep(0.04)
            _enviar(_mouse(u))
            time.sleep(0.06)
        for m in reversed(mods):
            _tecla_vk(VK[m], False)

    def arrastar(self, x0, y0, x1, y1, passos: int = 20, botao: str = "esq",
                 mods: tuple[str, ...] = (), pausa: float = 0.02,
                 foto_segurando: str | None = None) -> None:
        self.mover(x0, y0)
        d, u = {"esq": (LDOWN, LUP), "dir": (RDOWN, RUP), "meio": (MDOWN, MUP)}[botao]
        for m in mods:
            _tecla_vk(VK[m], True)
        _enviar(_mouse(d))
        time.sleep(0.08)
        for k in range(1, passos + 1):
            t = k / passos
            self.foco()
            self._ir(round(x0 + (x1 - x0) * t), round(y0 + (y1 - y0) * t))
            time.sleep(pausa)
        time.sleep(0.15)
        if foto_segurando:  # mostra o indicador de soltura antes de soltar
            self._ir(x1, y1 + 1)
            self._ir(x1, y1)
            self.foto(foto_segurando, estabilizar=0.4)
        _enviar(_mouse(u))
        for m in reversed(mods):
            _tecla_vk(VK[m], False)
        time.sleep(0.1)

    def rolar(self, x: int, y: int, ticks: int) -> None:
        self.mover(x, y)
        passo = 120 if ticks > 0 else -120
        for _ in range(abs(ticks)):
            _enviar(_mouse(WHEEL, dado=passo))
            time.sleep(0.05)

    def tecla(self, combo: str) -> None:
        self.foco()
        partes = [p.strip().lower() for p in combo.split("+")]
        vks = []
        for p in partes:
            if p in VK:
                vks.append(VK[p])
            elif len(p) == 1:
                vks.append(user32.VkKeyScanW(ord(p)) & 0xFF)
            else:
                raise ValueError(f"tecla desconhecida: {p}")
        for v in vks:
            _tecla_vk(v, True)
            time.sleep(0.02)
        time.sleep(0.05)
        for v in reversed(vks):
            _tecla_vk(v, False)
            time.sleep(0.02)

    def digitar(self, texto: str) -> None:
        self.foco()
        for ch in texto:
            for up in (0, K_UP):
                i = INPUT(type=INPUT_KEYBOARD)
                i.u.ki = KEYBDINPUT(0, ord(ch), K_UNI | up, 0, 0)
                _enviar(i)
            time.sleep(0.02)

    # ------------------------------------------------------------ captura
    def foto(self, nome: str | None = None, estabilizar: float = 0.8) -> Path:
        """Espera a tela parar de mudar e grava a área cliente em PNG.

        O Fenkra só redesenha com entrada nova: o resultado de um clique pode aparecer só no
        evento seguinte. Por isso, antes de capturar, o cursor anda 1 px e volta (FK_CUTUCAR=0
        desliga, para provar o atraso)."""
        time.sleep(estabilizar)
        if os.environ.get("FK_CUTUCAR", "1") != "0" and na_frente(self.h):
            p = W.POINT()
            user32.GetCursorPos(ctypes.byref(p))
            ox, oy, w, hh = cliente(self.h)
            if ox <= p.x < ox + w - 1 and oy <= p.y < oy + hh:
                for dx in (1, 0):
                    ax, ay = _abs(p.x + dx, p.y)
                    _enviar(_mouse(MOVE | ABS | VDESK, ax, ay))
                    time.sleep(0.08)
            time.sleep(0.3)
        ant = None
        for _ in range(8):
            w, hh, px = capturar(self.h)
            assin = zlib.crc32(px[::97])
            if assin == ant:
                break
            ant = assin
            time.sleep(0.4)
        self.passo += 1
        st = _estado()
        st["passo"] = self.passo
        _gravar_estado(st)
        arq = self.saida / f"{self.passo:03d}_{nome or 'foto'}.png"
        gravar_png(arq, w, hh, px)
        print(arq)
        return arq


# ---------------------------------------------------------------- captura e PNG
def capturar(h: int) -> tuple[int, int, bytes]:
    """Copia a área cliente como está na tela (o que a pessoa vê). Devolve RGB."""
    ox, oy, w, hh = cliente(h)
    tela = user32.GetDC(None)
    mem = gdi32.CreateCompatibleDC(tela)
    bmp = gdi32.CreateCompatibleBitmap(tela, w, hh)
    velho = gdi32.SelectObject(mem, bmp)
    ok = gdi32.BitBlt(mem, 0, 0, w, hh, tela, ox, oy, 0x00CC0020 | 0x40000000)
    if not ok:  # alternativa: pede a janela desenhar no DC
        user32.PrintWindow(h, mem, 3)
    cab = struct.pack("<IiiHHIIiiII", 40, w, -hh, 1, 32, 0, 0, 0, 0, 0, 0)
    buf = ctypes.create_string_buffer(w * hh * 4)
    gdi32.GetDIBits(mem, bmp, 0, hh, buf, cab, 0)
    gdi32.SelectObject(mem, velho)
    gdi32.DeleteObject(bmp)
    gdi32.DeleteDC(mem)
    user32.ReleaseDC(None, tela)
    bgra = buf.raw
    rgb = bytearray(w * hh * 3)
    rgb[0::3] = bgra[2::4]
    rgb[1::3] = bgra[1::4]
    rgb[2::3] = bgra[0::4]
    return w, hh, bytes(rgb)


def gravar_png(arq: Path, w: int, h: int, rgb: bytes) -> None:
    linha = w * 3
    cru = b"".join(b"\x00" + rgb[y * linha:(y + 1) * linha] for y in range(h))

    def bloco(tipo: bytes, dado: bytes) -> bytes:
        return struct.pack(">I", len(dado)) + tipo + dado + struct.pack(
            ">I", zlib.crc32(tipo + dado) & 0xFFFFFFFF)

    arq = Path(arq)
    arq.parent.mkdir(parents=True, exist_ok=True)
    arq.write_bytes(b"\x89PNG\r\n\x1a\n" + bloco(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 2, 0, 0, 0))
                    + bloco(b"IDAT", zlib.compress(cru, 6)) + bloco(b"IEND", b""))


def ler_png(arq: Path) -> tuple[int, int, bytes]:
    """Lê os PNG RGB 8 bits gravados aqui (filtros 0 a 4)."""
    d = Path(arq).read_bytes()
    pos, idat = 8, b""
    w = h = bpp = 0
    while pos < len(d):
        n = struct.unpack(">I", d[pos:pos + 4])[0]
        tipo = d[pos + 4:pos + 8]
        dado = d[pos + 8:pos + 8 + n]
        if tipo == b"IHDR":
            w, h, prof, cor = struct.unpack(">IIBB", dado[:10])
            if prof != 8 or cor not in (2, 6):
                raise ValueError("só PNG RGB/RGBA de 8 bits")
            bpp = 3 if cor == 2 else 4
        elif tipo == b"IDAT":
            idat += dado
        pos += 12 + n
    cru = zlib.decompress(idat)
    linha = w * bpp
    saida = bytearray(w * h * 3)
    ant = bytearray(linha)
    p = 0
    for y in range(h):
        f = cru[p]
        cur = bytearray(cru[p + 1:p + 1 + linha])
        p += 1 + linha
        for i in range(linha):
            a = cur[i - bpp] if i >= bpp else 0
            b = ant[i]
            c = ant[i - bpp] if i >= bpp else 0
            if f == 1:
                cur[i] = (cur[i] + a) & 255
            elif f == 2:
                cur[i] = (cur[i] + b) & 255
            elif f == 3:
                cur[i] = (cur[i] + (a + b) // 2) & 255
            elif f == 4:
                pa, pb, pc = abs(b - c), abs(a - c), abs(a + b - 2 * c)
                cur[i] = (cur[i] + (a if pa <= pb and pa <= pc else b if pb <= pc else c)) & 255
        ant = cur
        if bpp == 3:
            saida[y * w * 3:(y + 1) * w * 3] = cur
        else:
            r = saida[y * w * 3:(y + 1) * w * 3]
            r[0::3], r[1::3], r[2::3] = cur[0::4], cur[1::4], cur[2::4]
            saida[y * w * 3:(y + 1) * w * 3] = r
    return w, h, bytes(saida)


def recortar(arq, x0, y0, x1, y1, saida, escala: int = 1) -> None:
    w, h, px = ler_png(arq)
    x0, y0, x1, y1 = max(0, x0), max(0, y0), min(w, x1), min(h, y1)
    cw, ch = x1 - x0, y1 - y0
    out = bytearray()
    for y in range(ch):
        lin = px[((y0 + y) * w + x0) * 3:((y0 + y) * w + x1) * 3]
        if escala > 1:
            lin = b"".join(lin[i:i + 3] * escala for i in range(0, len(lin), 3))
        out += lin * escala
    gravar_png(saida, cw * escala, ch * escala, bytes(out))


def reduzir(arq, saida, fator: int = 2) -> None:
    w, h, px = ler_png(arq)
    nw, nh = w // fator, h // fator
    out = bytearray(nw * nh * 3)
    for y in range(nh):
        base = y * fator * w
        for x in range(nw):
            i = (base + x * fator) * 3
            out[(y * nw + x) * 3:(y * nw + x) * 3 + 3] = px[i:i + 3]
    gravar_png(saida, nw, nh, bytes(out))


def juntar(saida, arqs: list[str], sep: int = 6) -> None:
    """Põe capturas lado a lado (antes e depois) numa imagem só."""
    imgs = [ler_png(a) for a in arqs]
    alt = max(h for _, h, _ in imgs)
    larg = sum(w for w, _, _ in imgs) + sep * (len(imgs) - 1)
    out = bytearray([255]) * (larg * alt * 3)
    x = 0
    for w, h, px in imgs:
        for y in range(h):
            out[(y * larg + x) * 3:(y * larg + x + w) * 3] = px[y * w * 3:(y + 1) * w * 3]
        x += w + sep
    gravar_png(saida, larg, alt, bytes(out))


# ---------------------------------------------------------------- abrir e fechar
def abrir(exe: str, appdata: str, saida: str, args: list[str] | None = None) -> None:
    env = dict(os.environ)
    for k in list(env):
        if k.startswith("FENKRA_"):
            del env[k]  # usuário de verdade: nada de demonstração
    Path(appdata).mkdir(parents=True, exist_ok=True)
    env["APPDATA"] = appdata
    env["LOCALAPPDATA"] = appdata
    p = subprocess.Popen([exe] + (args or []), env=env, cwd=str(Path(exe).parent))
    _gravar_estado({"pid": p.pid, "saida": saida, "passo": 0})
    fim = time.time() + 90
    h = 0
    while time.time() < fim:
        hs = janelas_do_processo(p.pid)
        if hs:
            h = hs[0]
            break
        time.sleep(0.5)
    if not h:
        raise SystemExit("o Fenkra não abriu janela em 90 s")
    st = _estado()
    st["hwnd"] = h
    _gravar_estado(st)
    time.sleep(2)
    trazer(h)
    maximizar(h)
    trazer(h)
    Fenkra().foto("abertura", estabilizar=3)


def fechar() -> None:
    st = _estado()
    h = st.get("hwnd")
    if h and user32.IsWindow(h):
        user32.PostMessageW(h, 0x0010, 0, 0)  # WM_CLOSE
        time.sleep(3)
    pid = st.get("pid")
    if pid:
        subprocess.run(["taskkill", "/PID", str(pid), "/F"], capture_output=True)


# ---------------------------------------------------------------- roteiro
def rodar_roteiro(arq: str) -> None:
    dados = json.loads(Path(arq).read_text(encoding="utf-8"))
    fk = Fenkra(dados.get("saida"))
    for i, p in enumerate(dados["passos"]):
        a = p["acao"]
        if a == "clicar":
            fk.clicar(p["x"], p["y"], p.get("botao", "esq"), mods=tuple(p.get("mods", ())))
        elif a == "duplo":
            fk.clicar(p["x"], p["y"], duplo=True)
        elif a == "direito":
            fk.clicar(p["x"], p["y"], "dir")
        elif a == "mover":
            fk.mover(p["x"], p["y"])
        elif a == "arrastar":
            fk.arrastar(p["x0"], p["y0"], p["x1"], p["y1"], p.get("passos", 20),
                        p.get("botao", "esq"), tuple(p.get("mods", ())),
                        foto_segurando=p.get("segurando"))
        elif a == "rolar":
            fk.rolar(p["x"], p["y"], p["ticks"])
        elif a == "tecla":
            fk.tecla(p["tecla"])
        elif a == "digitar":
            fk.digitar(p["texto"])
        elif a == "esperar":
            time.sleep(p["s"])
        elif a != "foto":
            raise ValueError(f"ação desconhecida: {a}")
        time.sleep(p.get("esperar", 0))
        if p.get("foto", True):
            fk.foto(p.get("nome", f"{a}{i}"))


def main(argv=None) -> int:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest="cmd", required=True)
    s = sub.add_parser("abrir"); s.add_argument("--exe", required=True); s.add_argument("--appdata", required=True)
    s.add_argument("--saida", default="capturas"); s.add_argument("args", nargs="*")
    s = sub.add_parser("foto"); s.add_argument("nome", nargs="?")
    for nome in ("clicar", "mover"):
        s = sub.add_parser(nome); s.add_argument("x", type=int); s.add_argument("y", type=int)
        s.add_argument("--botao", default="esq"); s.add_argument("--duplo", action="store_true")
        s.add_argument("--esperar", type=float, default=0.0); s.add_argument("--nome")
        s.add_argument("--sem-foto", action="store_true"); s.add_argument("--mods", default="")
    s = sub.add_parser("arrastar")
    for c in ("x0", "y0", "x1", "y1"):
        s.add_argument(c, type=int)
    s.add_argument("--passos", type=int, default=20); s.add_argument("--botao", default="esq")
    s.add_argument("--mods", default=""); s.add_argument("--nome"); s.add_argument("--esperar", type=float, default=0.0)
    s.add_argument("--segurando", help="nome da captura tirada antes de soltar o botão")
    s = sub.add_parser("rolar"); s.add_argument("x", type=int); s.add_argument("y", type=int)
    s.add_argument("ticks", type=int); s.add_argument("--nome")
    s = sub.add_parser("tecla"); s.add_argument("combo"); s.add_argument("--nome"); s.add_argument("--esperar", type=float, default=0.0)
    s = sub.add_parser("digitar"); s.add_argument("texto"); s.add_argument("--nome")
    s = sub.add_parser("roteiro"); s.add_argument("arquivo")
    s = sub.add_parser("recorte"); s.add_argument("png")
    for c in ("x0", "y0", "x1", "y1"):
        s.add_argument(c, type=int)
    s.add_argument("saida"); s.add_argument("--escala", type=int, default=1)
    s = sub.add_parser("reduzir"); s.add_argument("png"); s.add_argument("saida"); s.add_argument("--fator", type=int, default=2)
    s = sub.add_parser("juntar"); s.add_argument("saida"); s.add_argument("pngs", nargs="+")
    sub.add_parser("fechar")
    a = ap.parse_args(argv)

    try:
        if a.cmd == "abrir":
            abrir(a.exe, a.appdata, a.saida, a.args)
        elif a.cmd == "fechar":
            fechar()
        elif a.cmd == "recorte":
            recortar(a.png, a.x0, a.y0, a.x1, a.y1, a.saida, a.escala)
        elif a.cmd == "juntar":
            juntar(a.saida, a.pngs)
        elif a.cmd == "reduzir":
            reduzir(a.png, a.saida, a.fator)
        elif a.cmd == "roteiro":
            rodar_roteiro(a.arquivo)
        else:
            fk = Fenkra()
            if a.cmd == "foto":
                fk.foto(a.nome, estabilizar=0.3)
                return 0
            if a.cmd == "clicar":
                fk.clicar(a.x, a.y, a.botao, a.duplo, tuple(m for m in a.mods.split("+") if m))
            elif a.cmd == "mover":
                fk.mover(a.x, a.y)
            elif a.cmd == "arrastar":
                fk.arrastar(a.x0, a.y0, a.x1, a.y1, a.passos, a.botao,
                            tuple(m for m in a.mods.split("+") if m),
                            foto_segurando=a.segurando)
            elif a.cmd == "rolar":
                fk.rolar(a.x, a.y, a.ticks)
            elif a.cmd == "tecla":
                fk.tecla(a.combo)
            elif a.cmd == "digitar":
                fk.digitar(a.texto)
            time.sleep(getattr(a, "esperar", 0.0) or 0.0)
            if not getattr(a, "sem_foto", False):
                fk.foto(a.nome or a.cmd)
    except SemFoco as e:
        print(f"[foco] {e}; encerrando", file=sys.stderr)
        return 3
    return 0


if __name__ == "__main__":
    sys.exit(main())
