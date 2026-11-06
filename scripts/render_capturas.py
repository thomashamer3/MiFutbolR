"""Convierte las capturas JSON de la TUI (examples/capturar_tui.rs) a PNG."""
import json, os
from PIL import Image, ImageDraw, ImageFont

BASE = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
SRC = os.path.join(BASE, "target", "capturas")
DST = os.path.join(BASE, "docs", "img")
os.makedirs(DST, exist_ok=True)

FUENTE = r"C:\Windows\Fonts\CascadiaMono.ttf"
FUENTE_BOLD = r"C:\Windows\Fonts\CascadiaMono-Bold.ttf"
if not os.path.exists(FUENTE_BOLD):
    FUENTE_BOLD = FUENTE
TAM = 15
f_normal = ImageFont.truetype(FUENTE, TAM)
f_bold = ImageFont.truetype(FUENTE_BOLD, TAM)
CW = int(round(f_normal.getlength("M"))) + 1
CH = 19
MARGEN = 18
BARRA = 34

def hexa(c):
    return tuple(c)

for nombre in ["menu", "partidos", "estadisticas", "financiamiento"]:
    with open(os.path.join(SRC, nombre + ".json"), encoding="utf-8") as f:
        data = json.load(f)
    ancho = data["width"] * CW + MARGEN * 2
    alto = data["height"] * CH + MARGEN * 2 + BARRA
    img = Image.new("RGB", (ancho, alto), (8, 18, 15))
    d = ImageDraw.Draw(img)
    # Marco tipo terminal
    d.rounded_rectangle([1, 1, ancho - 2, alto - 2], radius=14, fill=(12, 16, 15), outline=(38, 52, 46), width=2)
    d.rounded_rectangle([1, 1, ancho - 2, BARRA], radius=14, fill=(20, 27, 24), outline=None)
    for i, col in enumerate([(235, 95, 87), (245, 189, 79), (98, 197, 84)]):
        d.ellipse([12 + i * 20, 12, 24 + i * 20, 24], fill=col)
    d.text((80, 8), "MiFutbolRS  ·  TUI  ·  v0.1", font=f_normal, fill=(150, 174, 160))

    y0 = MARGEN + BARRA
    for y in range(data["height"]):
        for x in range(data["width"]):
            celda = data["celdas"][y * data["width"] + x]
            px = MARGEN + x * CW
            py = y0 + y * CH
            d.rectangle([px, py, px + CW, py + CH], fill=hexa(celda["bg"]))
    for y in range(data["height"]):
        for x in range(data["width"]):
            celda = data["celdas"][y * data["width"] + x]
            s = celda["s"]
            if not s or s == " ":
                continue
            px = MARGEN + x * CW
            py = y0 + y * CH
            fuente = f_bold if celda.get("bold") else f_normal
            fg = hexa(celda["fg"])
            if celda.get("dim"):
                fg = tuple(int(v * 0.65) for v in fg)
            d.text((px, py), s, font=fuente, fill=fg)

    salida = os.path.join(DST, nombre + ".png")
    img.save(salida)
    print(salida, img.size)
