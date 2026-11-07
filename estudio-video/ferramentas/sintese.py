"""Síntese sonora básica do estúdio (numpy). Tudo o que sai daqui é original e livre de direitos."""
import wave
from pathlib import Path

import numpy as np

SR = 48000
_rng = np.random.default_rng(7)


def t(dur):
    return np.arange(int(SR * dur)) / SR


def ruido(dur, rng=None):
    return (rng or _rng).uniform(-1, 1, int(SR * dur))


def env(n, a=0.005, r=0.2, curva=4.0):
    """Envelope ataque linear + queda exponencial, em amostras."""
    x = np.ones(n)
    na = max(1, int(a * SR))
    x[:na] = np.linspace(0, 1, na)
    nr = n - na
    if nr > 0:
        x[na:] = np.exp(-curva * np.linspace(0, 1, nr) * (len(x) / SR) / max(r, 1e-3))
    return x


def adsr(n, a=0.01, d=0.1, s=0.7, r=0.2):
    x = np.zeros(n)
    na, nd, nr = int(a * SR), int(d * SR), int(r * SR)
    ns = max(0, n - na - nd - nr)
    partes = [np.linspace(0, 1, na, endpoint=False), np.linspace(1, s, nd, endpoint=False),
              np.full(ns, s), np.linspace(s, 0, nr)]
    y = np.concatenate(partes)[:n]
    x[:len(y)] = y
    return x


def passa_baixa(x, corte):
    """Passa-baixa de um polo com corte variável (escalar ou vetor em Hz)."""
    corte = np.broadcast_to(np.asarray(corte, dtype=float), x.shape)
    a = 1 - np.exp(-2 * np.pi * np.clip(corte, 10, SR / 2.2) / SR)
    y = np.empty_like(x)
    acc = 0.0
    for i in range(len(x)):
        acc += a[i] * (x[i] - acc)
        y[i] = acc
    return y


def passa_baixa_fixo(x, corte, ordem=2):
    """Passa-baixa rápido por FFT (corte fixo)."""
    f = np.fft.rfftfreq(len(x), 1 / SR)
    h = 1 / np.sqrt(1 + (f / corte) ** (2 * ordem))
    return np.fft.irfft(np.fft.rfft(x) * h, len(x))


def passa_alta_fixo(x, corte, ordem=2):
    f = np.fft.rfftfreq(len(x), 1 / SR)
    h = 1 / np.sqrt(1 + (corte / np.maximum(f, 1e-6)) ** (2 * ordem))
    return np.fft.irfft(np.fft.rfft(x) * h, len(x))


def seno_varredura(f0, f1, dur, curva="exp"):
    tt = t(dur)
    if curva == "exp":
        f = f0 * (f1 / f0) ** (tt / dur)
    else:
        f = f0 + (f1 - f0) * tt / dur
    return np.sin(2 * np.pi * np.cumsum(f) / SR)


def serra(freq, dur, detune=0.0):
    tt = t(dur)
    y = np.zeros_like(tt)
    for d in (-detune, 0.0, detune) if detune else (0.0,):
        ph = (tt * freq * (1 + d)) % 1.0
        y += 2 * ph - 1
    return y / (3 if detune else 1)


def reverb(x, tamanho=0.35, mix=0.25, seed=3):
    """Reverb por convolução com cauda de ruído decrescente (sintética)."""
    rng = np.random.default_rng(seed)
    n = int(SR * tamanho * 4)
    ir = rng.uniform(-1, 1, n) * np.exp(-np.linspace(0, 8, n))
    ir = passa_baixa_fixo(ir, 6000)
    ir /= np.sqrt(np.sum(ir ** 2)) + 1e-9
    m = len(x) + n - 1
    k = 1 << (m - 1).bit_length()
    w = np.fft.irfft(np.fft.rfft(x, k) * np.fft.rfft(ir, k), k)[:m]
    out = np.concatenate([x, np.zeros(n - 1)])
    return (1 - mix) * out + mix * w


def normalizar(x, pico_db=-1.0):
    p = np.max(np.abs(x)) + 1e-9
    return x / p * 10 ** (pico_db / 20)


def fade(x, ini=0.003, fim=0.01):
    a, b = int(ini * SR), int(fim * SR)
    y = x.copy()
    if a:
        y[:a] *= np.linspace(0, 1, a)
    if b:
        y[-b:] *= np.linspace(1, 0, b)
    return y


def salvar(caminho, x, estereo=None):
    """Salva WAV 16 bits 48 kHz. `estereo`: (esq, dir) opcional."""
    caminho = Path(caminho)
    caminho.parent.mkdir(parents=True, exist_ok=True)
    if estereo is None:
        dados = np.stack([x, x], axis=1)
    else:
        dados = np.stack(estereo, axis=1)
    dados = np.clip(dados, -1, 1)
    with wave.open(str(caminho), "wb") as w:
        w.setnchannels(2)
        w.setsampwidth(2)
        w.setframerate(SR)
        w.writeframes((dados * 32767).astype("<i2").tobytes())
