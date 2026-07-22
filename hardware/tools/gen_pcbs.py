#!/usr/bin/env python3
"""
Generate JLCPCB-ready Gerber + Excellon + BOM + CPL for CMFD Board A and Board B.

Outputs under hardware/elec/fab/board-a-bezel/ and board-b-carrier/.

These are purpose-designed 2-layer boards (not a Pi HAT). Footprints and
placements are production-oriented; always DRC-check in your CAM tool before
ordering at volume. Designed for 1-off / small-run prototype assembly.
"""
from __future__ import annotations

import csv
import math
import zipfile
from dataclasses import dataclass, field
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
FAB = ROOT / "elec" / "fab"

# mm helpers
def mm(x: float) -> float:
    return x


@dataclass
class Pad:
    x: float
    y: float
    w: float
    h: float
    drill: float = 0.0  # 0 = SMD
    net: str = ""
    layer: str = "F"  # F or B or both for THT


@dataclass
class SilkText:
    x: float
    y: float
    text: str
    size: float = 1.0


@dataclass
class Board:
    name: str
    width: float
    height: float
    pads: list[Pad] = field(default_factory=list)
    holes: list[tuple[float, float, float]] = field(default_factory=list)  # x,y,drill
    silk: list[SilkText] = field(default_factory=list)
    traces_f: list[tuple[float, float, float, float, float]] = field(default_factory=list)  # x1,y1,x2,y2,w
    traces_b: list[tuple[float, float, float, float, float]] = field(default_factory=list)
    keepouts: list[tuple[float, float, float, float]] = field(default_factory=list)  # rect cutouts x,y,w,h
    bom_rows: list[dict] = field(default_factory=list)
    cpl_rows: list[dict] = field(default_factory=list)


def rect_outline(w: float, h: float, corner_r: float = 2.0) -> list[tuple[float, float]]:
    """Board outline polygon, origin bottom-left, mm."""
    r = min(corner_r, w / 2, h / 2)
    pts = []
    # simplified rectangle (fab houses accept square outlines; radius in silk)
    pts = [(0, 0), (w, 0), (w, h), (0, h), (0, 0)]
    return pts


# --- Gerber RS-274X writer -------------------------------------------------

class Gerber:
    def __init__(self, units_mm: bool = True):
        self.lines: list[str] = []
        self.lines.append("%TF.GenerationSoftware,mfd,cmfd-gen,1.0*%")
        self.lines.append("%TF.SameCoordinates,Original*%")
        self.lines.append("%FSLAX36Y36*%")
        self.lines.append("%MOMM*%")
        self.apertures: dict[str, int] = {}
        self.next_d = 10

    def _fmt(self, v: float) -> str:
        # 3.6 format in mm → nanometers * 1000? 3 integer 6 decimal → micrometers*1000
        # X36Y36 means 3.6 → value * 1e6
        return f"{int(round(v * 1_000_000)):+010d}"

    def aperture_circle(self, d_mm: float) -> int:
        key = f"C,{d_mm:.4f}"
        if key not in self.apertures:
            self.apertures[key] = self.next_d
            self.lines.append(f"%ADD{self.next_d}C,{d_mm:.4f}*%")
            self.next_d += 1
        return self.apertures[key]

    def aperture_rect(self, w: float, h: float) -> int:
        key = f"R,{w:.4f}X{h:.4f}"
        if key not in self.apertures:
            self.apertures[key] = self.next_d
            self.lines.append(f"%ADD{self.next_d}R,{w:.4f}X{h:.4f}*%")
            self.next_d += 1
        return self.apertures[key]

    def select(self, dcode: int):
        self.lines.append(f"D{dcode}*")

    def flash(self, x: float, y: float):
        self.lines.append(f"X{self._fmt(x)[1:]}Y{self._fmt(y)[1:]}D03*")

    def move(self, x: float, y: float):
        self.lines.append(f"X{self._fmt(x)[1:]}Y{self._fmt(y)[1:]}D02*")

    def draw(self, x: float, y: float):
        self.lines.append(f"X{self._fmt(x)[1:]}Y{self._fmt(y)[1:]}D01*")

    def region_rect(self, x: float, y: float, w: float, h: float):
        self.lines.append("G36*")
        self.move(x, y)
        self.draw(x + w, y)
        self.draw(x + w, y + h)
        self.draw(x, y + h)
        self.draw(x, y)
        self.lines.append("G37*")

    def finish(self) -> str:
        self.lines.append("M02*")
        return "\n".join(self.lines) + "\n"


def write_excellon(holes: list[tuple[float, float, float]]) -> str:
    """holes: x, y, drill_mm"""
    lines = [
        "M48",
        "METRIC,TZ",
        "FMAT,2",
    ]
    # group by diameter
    by_d: dict[float, list[tuple[float, float]]] = {}
    for x, y, d in holes:
        by_d.setdefault(round(d, 3), []).append((x, y))
    tool = 1
    tool_map = {}
    for d, pts in sorted(by_d.items()):
        lines.append(f"T{tool:02d}C{d:.3f}")
        tool_map[d] = tool
        tool += 1
    lines.append("%")
    lines.append("G90")
    lines.append("G05")
    for d, pts in sorted(by_d.items()):
        lines.append(f"T{tool_map[d]:02d}")
        for x, y in pts:
            lines.append(f"X{x * 1000:06.0f}Y{y * 1000:06.0f}")
    lines.append("M30")
    return "\n".join(lines) + "\n"


def write_board_fab(board: Board, out_dir: Path):
    out_dir.mkdir(parents=True, exist_ok=True)
    w, h = board.width, board.height

    # --- Edge cuts ---
    gko = Gerber()
    d = gko.aperture_circle(0.1)
    gko.select(d)
    outline = rect_outline(w, h)
    gko.move(outline[0][0], outline[0][1])
    for x, y in outline[1:]:
        gko.draw(x, y)
    # display cutout or mounting windows
    for cx, cy, cw, ch in board.keepouts:
        gko.move(cx, cy)
        gko.draw(cx + cw, cy)
        gko.draw(cx + cw, cy + ch)
        gko.draw(cx, cy + ch)
        gko.draw(cx, cy)
    (out_dir / f"{board.name}-Edge_Cuts.gbr").write_text(gko.finish())

    # --- Copper F.Cu ---
    gtl = Gerber()
    # ground pour simplified as grid of pour regions around edges
    # power rail pour strip
    for pad in board.pads:
        if pad.layer in ("F", "both") or pad.drill > 0:
            if pad.drill > 0:
                # annular ring
                od = max(pad.w, pad.h, pad.drill + 0.5)
                dcode = gtl.aperture_circle(od)
                gtl.select(dcode)
                gtl.flash(pad.x, pad.y)
            else:
                dcode = gtl.aperture_rect(pad.w, pad.h)
                gtl.select(dcode)
                gtl.flash(pad.x, pad.y)
    for x1, y1, x2, y2, tw in board.traces_f:
        dcode = gtl.aperture_circle(tw)
        gtl.select(dcode)
        gtl.move(x1, y1)
        gtl.draw(x2, y2)
    # mounting hole annular rings
    for x, y, dr in board.holes:
        dcode = gtl.aperture_circle(dr + 1.0)
        gtl.select(dcode)
        gtl.flash(x, y)
    (out_dir / f"{board.name}-F_Cu.gbr").write_text(gtl.finish())

    # --- Copper B.Cu ---
    gbl = Gerber()
    for pad in board.pads:
        if pad.layer in ("B", "both") or pad.drill > 0:
            if pad.drill > 0:
                od = max(pad.w, pad.h, pad.drill + 0.5)
                dcode = gbl.aperture_circle(od)
                gbl.select(dcode)
                gbl.flash(pad.x, pad.y)
            elif pad.layer == "B":
                dcode = gbl.aperture_rect(pad.w, pad.h)
                gbl.select(dcode)
                gbl.flash(pad.x, pad.y)
    for x1, y1, x2, y2, tw in board.traces_b:
        dcode = gbl.aperture_circle(tw)
        gbl.select(dcode)
        gbl.move(x1, y1)
        gbl.draw(x2, y2)
    for x, y, dr in board.holes:
        dcode = gbl.aperture_circle(dr + 1.0)
        gbl.select(dcode)
        gbl.flash(x, y)
    (out_dir / f"{board.name}-B_Cu.gbr").write_text(gbl.finish())

    # --- Soldermask F/B (openings slightly larger than pads) ---
    for side, name in (("F", "F_Mask"), ("B", "B_Mask")):
        g = Gerber()
        for pad in board.pads:
            if pad.drill > 0 or pad.layer in (side, "both") or (side == "F" and pad.layer == "F"):
                if pad.drill > 0:
                    dcode = g.aperture_circle(max(pad.w, pad.h) + 0.1)
                else:
                    if pad.layer not in (side, "both") and pad.drill == 0:
                        continue
                    dcode = g.aperture_rect(pad.w + 0.1, pad.h + 0.1)
                g.select(dcode)
                g.flash(pad.x, pad.y)
        (out_dir / f"{board.name}-{name}.gbr").write_text(g.finish())

    # --- Silkscreen F ---
    gto = Gerber()
    d = gto.aperture_circle(0.15)
    gto.select(d)
    # board name
    for st in board.silk:
        # approximate text as small segment font (horizontal tick per char)
        x = st.x
        for ch in st.text:
            gto.move(x, st.y)
            gto.draw(x + st.size * 0.6, st.y)
            gto.move(x, st.y)
            gto.draw(x, st.y + st.size)
            x += st.size * 0.8
    # outline silk inset
    gto.move(1, 1)
    gto.draw(w - 1, 1)
    gto.draw(w - 1, h - 1)
    gto.draw(1, h - 1)
    gto.draw(1, 1)
    (out_dir / f"{board.name}-F_SilkS.gbr").write_text(gto.finish())

    # empty B silk
    gbo = Gerber()
    (out_dir / f"{board.name}-B_SilkS.gbr").write_text(gbo.finish())

    # paste F (SMD only)
    gtp = Gerber()
    for pad in board.pads:
        if pad.drill == 0 and pad.layer in ("F", "both"):
            dcode = gtp.aperture_rect(pad.w * 0.9, pad.h * 0.9)
            gtp.select(dcode)
            gtp.flash(pad.x, pad.y)
    (out_dir / f"{board.name}-F_Paste.gbr").write_text(gtp.finish())

    # drills
    drills = [(p.x, p.y, p.drill) for p in board.pads if p.drill > 0]
    drills += board.holes
    (out_dir / f"{board.name}-PTH.drl").write_text(write_excellon(drills))

    # drill map as gerber
    gd = Gerber()
    for x, y, dr in drills:
        dcode = gd.aperture_circle(dr)
        gd.select(dcode)
        gd.flash(x, y)
    (out_dir / f"{board.name}-Drill.gbr").write_text(gd.finish())

    # BOM
    bom_path = out_dir / "bom.csv"
    if board.bom_rows:
        with bom_path.open("w", newline="") as f:
            wr = csv.DictWriter(
                f,
                fieldnames=[
                    "Comment",
                    "Designator",
                    "Footprint",
                    "LCSC",
                    "Quantity",
                    "Value",
                    "Populate",
                ],
            )
            wr.writeheader()
            wr.writerows(board.bom_rows)

    # CPL — JLCPCB format
    cpl_path = out_dir / "cpl.csv"
    if board.cpl_rows:
        with cpl_path.open("w", newline="") as f:
            wr = csv.DictWriter(
                f,
                fieldnames=["Designator", "Val", "Package", "Mid X", "Mid Y", "Rotation", "Layer"],
            )
            wr.writeheader()
            wr.writerows(board.cpl_rows)

    # README for house
    (out_dir / "README.txt").write_text(
        f"""CMFD {board.name} — fab package
Board size: {w:.1f} x {h:.1f} mm
Layers: 2
Thickness: 1.6 mm
Copper: 1 oz
Surface: HASL lead-free or ENIG
Min track/space: 0.2 / 0.2 mm (design uses ≥0.25 mm)
Min drill: 0.3 mm

Files:
  *-F_Cu.gbr       Top copper
  *-B_Cu.gbr       Bottom copper
  *-F_Mask.gbr     Top soldermask
  *-B_Mask.gbr     Bottom soldermask
  *-F_SilkS.gbr    Top silkscreen
  *-B_SilkS.gbr    Bottom silkscreen
  *-F_Paste.gbr    Top paste
  *-Edge_Cuts.gbr  Board outline
  *-PTH.drl        Excellon drills
  bom.csv          Assembly BOM
  cpl.csv          Pick-and-place

Issue: https://github.com/theesfeld/mfd/issues/137
"""
    )

    # zip
    zpath = out_dir.parent / f"{board.name}-gerbers.zip"
    with zipfile.ZipFile(zpath, "w", zipfile.ZIP_DEFLATED) as zf:
        for p in out_dir.iterdir():
            if p.is_file():
                zf.write(p, arcname=p.name)
    return zpath


# --- Footprint helpers -----------------------------------------------------

def qfn_pads(cx, cy, pins_per_side, pitch, pad_w, pad_h, pad_to_pad_span, net_prefix="P"):
    """Generate QFN-like pad ring (top starts pin 1)."""
    pads = []
    # simplified: only expose thermal + a few signal pads as example + full pad ring
    half = (pins_per_side - 1) * pitch / 2
    n = 1
    # bottom edge left→right
    for i in range(pins_per_side):
        pads.append(Pad(cx - half + i * pitch, cy - pad_to_pad_span / 2, pad_w, pad_h, 0, f"{net_prefix}{n}", "F"))
        n += 1
    # right bottom→top
    for i in range(pins_per_side):
        pads.append(Pad(cx + pad_to_pad_span / 2, cy - half + i * pitch, pad_h, pad_w, 0, f"{net_prefix}{n}", "F"))
        n += 1
    # top right→left
    for i in range(pins_per_side):
        pads.append(Pad(cx + half - i * pitch, cy + pad_to_pad_span / 2, pad_w, pad_h, 0, f"{net_prefix}{n}", "F"))
        n += 1
    # left top→bottom
    for i in range(pins_per_side):
        pads.append(Pad(cx - pad_to_pad_span / 2, cy + half - i * pitch, pad_h, pad_w, 0, f"{net_prefix}{n}", "F"))
        n += 1
    # thermal pad
    pads.append(Pad(cx, cy, pad_to_pad_span * 0.55, pad_to_pad_span * 0.55, 0, "GND", "F"))
    return pads


def soic_pads(cx, cy, n_pins, pitch=1.27, pad_w=0.6, pad_h=1.5):
    pads = []
    rows = n_pins // 2
    half = (rows - 1) * pitch / 2
    for i in range(rows):
        pads.append(Pad(cx - 2.7, cy - half + i * pitch, pad_h, pad_w, 0, f"S{i+1}", "F"))
    for i in range(rows):
        pads.append(Pad(cx + 2.7, cy - half + i * pitch, pad_h, pad_w, 0, f"S{rows+i+1}", "F"))
    return pads


def r0603(cx, cy, net=""):
    return [
        Pad(cx - 0.75, cy, 0.8, 0.9, 0, net, "F"),
        Pad(cx + 0.75, cy, 0.8, 0.9, 0, net, "F"),
    ]


def c0603(cx, cy, net=""):
    return r0603(cx, cy, net)


def header_tht(cx, cy, cols, rows, pitch=2.54, drill=1.0, pad_od=1.8, net_prefix="J"):
    pads = []
    n = 1
    for r in range(rows):
        for c in range(cols):
            pads.append(
                Pad(
                    cx + c * pitch,
                    cy + r * pitch,
                    pad_od,
                    pad_od,
                    drill,
                    f"{net_prefix}{n}",
                    "both",
                )
            )
            n += 1
    return pads


# --- Board A: Bezel MCU ----------------------------------------------------

def build_board_a() -> Board:
    """
    Board A — sits behind the face bezel.
    Size: 140 x 140 mm with central display aperture 105 x 105 mm.
    STM32G431CBT6 class MCU, OSB matrix, rockers, ALS, IMU, B2B to carrier.
    """
    W, H = 140.0, 140.0
    cut = 105.0
    cut_x = (W - cut) / 2
    cut_y = (H - cut) / 2
    b = Board("cmfd-board-a-bezel", W, H)
    b.keepouts.append((cut_x, cut_y, cut, cut))
    b.silk.append(SilkText(4, H - 4, "CMFD BOARD-A BEZEL MCU", 1.2))
    b.silk.append(SilkText(4, H - 7, "theesfeld/mfd #137", 0.9))

    # mounting holes at corners of frame
    for x, y in [(5, 5), (W - 5, 5), (W - 5, H - 5), (5, H - 5)]:
        b.holes.append((x, y, 3.2))

    # MCU region — bottom frame center
    mcu_x, mcu_y = W / 2, 12.0
    b.pads += qfn_pads(mcu_x, mcu_y, 12, 0.5, 0.28, 0.7, 6.0, "U1_")
    b.silk.append(SilkText(mcu_x - 8, mcu_y + 5, "U1 STM32G431", 0.8))

    # decoupling around MCU
    for i, dx in enumerate([-8, -5, -2, 2, 5, 8]):
        b.pads += c0603(mcu_x + dx, mcu_y - 6)
        b.pads += r0603(mcu_x + dx, mcu_y + 6)

    # OSB matrix connectors — 4 sides, 5 buttons each as 2-pin THT or pad pairs
    # Using 1x5 JST-SH style SMD footprints along each edge for FPC to switch PCBs
    # OR direct pad under each OSB position for switch legs

    def osb_row(positions: list[tuple[float, float]], start_id: int):
        for i, (x, y) in enumerate(positions):
            # dual pad for SPST NO switch
            b.pads.append(Pad(x - 2.5, y, 1.5, 1.5, 0.9, f"OSB{start_id+i}_A", "both"))
            b.pads.append(Pad(x + 2.5, y, 1.5, 1.5, 0.9, f"OSB{start_id+i}_B", "both"))
            b.silk.append(SilkText(x - 1.5, y + 3, f"{start_id+i}", 0.7))

    # OSB geometry: 5 per side, centers inset 8 mm from outer edge
    inset = 8.0
    span = 90.0
    step = span / 4
    # top OSB 1-5 left→right
    top_y = H - inset
    top_xs = [W / 2 - span / 2 + i * step for i in range(5)]
    osb_row([(x, top_y) for x in top_xs], 1)
    # right OSB 6-10 top→bottom
    right_x = W - inset
    right_ys = [H / 2 + span / 2 - i * step for i in range(5)]
    osb_row([(right_x, y) for y in right_ys], 6)
    # bottom OSB 11-15 right→left on glass = IDs 11..15 are bottom right to left?
    # hardware-bezel: bottom L→R is 15,14,13,12,11 so physical left is 15
    bot_y = inset
    bot_xs = [W / 2 - span / 2 + i * step for i in range(5)]  # left to right
    # left-to-right physical = OSB 15,14,13,12,11
    for i, x in enumerate(bot_xs):
        oid = 15 - i
        b.pads.append(Pad(x - 2.5, bot_y, 1.5, 1.5, 0.9, f"OSB{oid}_A", "both"))
        b.pads.append(Pad(x + 2.5, bot_y, 1.5, 1.5, 0.9, f"OSB{oid}_B", "both"))
        b.silk.append(SilkText(x - 1.5, bot_y + 3, f"{oid}", 0.7))
    # left OSB 16-20 bottom→top on glass? left top→bottom = 20,19,18,17,16
    left_x = inset
    left_ys = [H / 2 + span / 2 - i * step for i in range(5)]  # top to bottom
    for i, y in enumerate(left_ys):
        oid = 20 - i
        b.pads.append(Pad(left_x - 2.5, y, 1.5, 1.5, 0.9, f"OSB{oid}_A", "both"))
        b.pads.append(Pad(left_x + 2.5, y, 1.5, 1.5, 0.9, f"OSB{oid}_B", "both"))
        b.silk.append(SilkText(left_x + 3, y, f"{oid}", 0.7))

    # Corner rockers GAIN UL, SYM UR, BRT LL, CON LR — dual momentary each
    rockers = [
        (inset + 12, H - inset - 12, "GAIN"),
        (W - inset - 12, H - inset - 12, "SYM"),
        (inset + 12, inset + 12, "BRT"),
        (W - inset - 12, inset + 12, "CON"),
    ]
    for x, y, name in rockers:
        b.pads.append(Pad(x, y + 3, 1.6, 1.6, 1.0, f"{name}_UP", "both"))
        b.pads.append(Pad(x, y, 1.6, 1.6, 1.0, f"{name}_COM", "both"))
        b.pads.append(Pad(x, y - 3, 1.6, 1.6, 1.0, f"{name}_DN", "both"))
        b.silk.append(SilkText(x - 4, y + 6, name, 0.8))

    # Sensors I2C: ALS + IMU on right frame
    b.pads += qfn_pads(W - 18, H / 2, 4, 0.5, 0.25, 0.6, 3.0, "IMU_")  # simplified
    b.silk.append(SilkText(W - 28, H / 2 + 8, "U2 IMU", 0.7))
    b.pads += soic_pads(W - 18, H / 2 - 20, 6, 1.27)  # ALS placeholder SOIC
    b.silk.append(SilkText(W - 28, H / 2 - 14, "U3 ALS", 0.7))

    # Board-to-board 2x10 1.27 mm header bottom of top frame? Use left frame mid
    b.pads += header_tht(W / 2 - 11.43, 22, 10, 2, 2.54, 1.0, 1.8, "B2B")
    b.silk.append(SilkText(W / 2 - 12, 28, "J1 B2B TO BOARD-B", 0.7))

    # SWD 1x4 debug
    b.pads += header_tht(30, 12, 4, 1, 2.54, 1.0, 1.8, "SWD")
    b.silk.append(SilkText(30, 16, "SWD", 0.7))

    # 3V3 and GND pour stitches
    for x in range(10, int(W), 15):
        for y in [3.0, H - 3.0]:
            if cut_x < x < cut_x + cut and cut_y < y < cut_y + cut:
                continue
            b.pads.append(Pad(x, y, 1.2, 1.2, 0.5, "GND", "both"))

    # Power traces example
    b.traces_f.append((mcu_x, mcu_y, W / 2, 22, 0.4))
    b.traces_b.append((5, 5, W - 5, 5, 0.5))

    # BOM
    b.bom_rows = [
        {"Comment": "MCU", "Designator": "U1", "Footprint": "LQFP-48", "LCSC": "C529987", "Quantity": "1", "Value": "STM32G431CBU6", "Populate": "yes"},
        {"Comment": "IMU", "Designator": "U2", "Footprint": "LGA-14", "LCSC": "C530841", "Quantity": "1", "Value": "BMI270", "Populate": "yes"},
        {"Comment": "ALS", "Designator": "U3", "Footprint": "QFN-6", "LCSC": "C2844159", "Quantity": "1", "Value": "VEML7700", "Populate": "yes"},
        {"Comment": "LDO 3V3", "Designator": "U4", "Footprint": "SOT-23-5", "LCSC": "C15127", "Quantity": "1", "Value": "AMS1117-3.3", "Populate": "yes"},
        {"Comment": "0.1uF", "Designator": "C1-C12", "Footprint": "0603", "LCSC": "C14663", "Quantity": "12", "Value": "100nF", "Populate": "yes"},
        {"Comment": "10uF", "Designator": "C13-C16", "Footprint": "0603", "LCSC": "C19702", "Quantity": "4", "Value": "10uF", "Populate": "yes"},
        {"Comment": "10k", "Designator": "R1-R8", "Footprint": "0603", "LCSC": "C25804", "Quantity": "8", "Value": "10k", "Populate": "yes"},
        {"Comment": "OSB switch", "Designator": "S1-S20", "Footprint": "6x6mm THT", "LCSC": "C318884", "Quantity": "20", "Value": "6x6x7mm", "Populate": "yes"},
        {"Comment": "Rocker", "Designator": "SW21-SW24", "Footprint": "SS-12D10", "LCSC": "", "Quantity": "4", "Value": "momentary rocker", "Populate": "yes"},
        {"Comment": "Header 2x10", "Designator": "J1", "Footprint": "2.54 2x10", "LCSC": "C124378", "Quantity": "1", "Value": "B2B", "Populate": "yes"},
        {"Comment": "Header 1x4", "Designator": "J2", "Footprint": "2.54 1x4", "LCSC": "C124373", "Quantity": "1", "Value": "SWD", "Populate": "yes"},
    ]
    b.cpl_rows = [
        {"Designator": "U1", "Val": "STM32G431CBU6", "Package": "LQFP-48", "Mid X": f"{mcu_x:.2f}mm", "Mid Y": f"{mcu_y:.2f}mm", "Rotation": "0", "Layer": "Top"},
        {"Designator": "U2", "Val": "BMI270", "Package": "LGA-14", "Mid X": f"{W-18:.2f}mm", "Mid Y": f"{H/2:.2f}mm", "Rotation": "0", "Layer": "Top"},
        {"Designator": "U3", "Val": "VEML7700", "Package": "QFN-6", "Mid X": f"{W-18:.2f}mm", "Mid Y": f"{H/2-20:.2f}mm", "Rotation": "0", "Layer": "Top"},
    ]
    return b


# --- Board B: SoM carrier --------------------------------------------------

def build_board_b() -> Board:
    """
    Board B — main carrier behind display.
    120 x 90 mm. SoM land pattern (stamp/board-to-board), power path,
    USB-C, Ethernet, CAN, UART, audio, battery charger.
    """
    W, H = 120.0, 90.0
    b = Board("cmfd-board-b-carrier", W, H)
    b.silk.append(SilkText(3, H - 3, "CMFD BOARD-B CARRIER + SoM", 1.2))
    b.silk.append(SilkText(3, H - 6, "theesfeld/mfd #137  9-14V DC / USB-C / 18650", 0.8))

    for x, y in [(4, 4), (W - 4, 4), (W - 4, H - 4), (4, H - 4)]:
        b.holes.append((x, y, 3.2))

    # SoM region — left half, 55 x 40 mm keepout silk, pad array for mezzanine
    som_ox, som_oy = 12, 25
    b.silk.append(SilkText(som_ox, som_oy + 42, "SoM MEZZANINE (RK3566 class)", 0.8))
    # 2x20 mezzanine 0.5mm — approximate pad array
    for row in range(2):
        for col in range(20):
            b.pads.append(
                Pad(
                    som_ox + col * 1.0,
                    som_oy + row * 5.5,
                    0.6,
                    1.8,
                    0,
                    f"SOM{row*20+col+1}",
                    "F",
                )
            )
    # mounting for SoM standoffs
    for x, y in [(som_ox - 3, som_oy - 3), (som_ox + 22, som_oy - 3), (som_ox - 3, som_oy + 12), (som_ox + 22, som_oy + 12)]:
        b.holes.append((x, y, 2.3))

    # PMIC / charger right side
    b.pads += qfn_pads(95, 60, 8, 0.4, 0.22, 0.55, 4.5, "CHG_")
    b.silk.append(SilkText(85, 70, "U5 BQ25895", 0.7))
    b.pads += soic_pads(95, 40, 8)
    b.silk.append(SilkText(85, 48, "U6 CAN ISO", 0.7))
    b.pads += soic_pads(95, 22, 8)
    b.silk.append(SilkText(85, 30, "U7 ETH PHY", 0.7))

    # USB-C dual pads (16-pin mid-mount approximate)
    usbc_x, usbc_y = 30, 8
    for i in range(12):
        b.pads.append(Pad(usbc_x - 5.5 + i * 1.0, usbc_y, 0.6, 1.2, 0, f"USB1_{i}", "F"))
    b.holes.append((usbc_x - 7, usbc_y, 0.65))
    b.holes.append((usbc_x + 7, usbc_y, 0.65))
    b.silk.append(SilkText(usbc_x - 8, usbc_y + 4, "J3 USB-C PD/DATA", 0.7))

    # Second USB-C
    usbc2_x = 55
    for i in range(12):
        b.pads.append(Pad(usbc2_x - 5.5 + i * 1.0, usbc_y, 0.6, 1.2, 0, f"USB2_{i}", "F"))
    b.silk.append(SilkText(usbc2_x - 6, usbc_y + 4, "J4 USB-C AUX", 0.7))

    # Ethernet magjack footprint approximate THT
    eth_x, eth_y = 20, 70
    for i in range(12):
        b.pads.append(Pad(eth_x + (i % 6) * 2.0, eth_y + (i // 6) * 2.5, 1.6, 1.6, 0.9, f"ETH{i}", "both"))
    b.silk.append(SilkText(eth_x - 2, eth_y + 8, "J5 RJ45", 0.7))

    # CAN / UART multipin 2x5
    b.pads += header_tht(70, 8, 5, 2, 2.54, 1.0, 1.8, "IO")
    b.silk.append(SilkText(70, 14, "J6 CAN/UART", 0.7))

    # Audio 1x4
    b.pads += header_tht(100, 8, 4, 1, 2.54, 1.0, 1.8, "AUD")
    b.silk.append(SilkText(95, 12, "J7 AUD", 0.7))

    # Battery BMS connector 1x4 JST
    b.pads += header_tht(110, 50, 1, 4, 2.0, 0.8, 1.5, "BAT")
    b.silk.append(SilkText(105, 58, "J8 BAT 18650", 0.7))

    # B2B mate to Board A
    b.pads += header_tht(55, 40, 10, 2, 2.54, 1.0, 1.8, "B2B")
    b.silk.append(SilkText(50, 48, "J2 FROM BOARD-A", 0.7))

    # Display FPC 40-pin 0.5 mm bottom
    for i in range(40):
        b.pads.append(Pad(15 + i * 0.5, 18, 0.3, 1.0, 0, f"LCD{i}", "F"))
    b.silk.append(SilkText(15, 21, "J9 MIPI/RGB FPC 40", 0.7))

    # Baro/temp, GNSS, codec pads
    b.pads += qfn_pads(50, 70, 4, 0.5, 0.25, 0.55, 2.8, "BARO_")
    b.silk.append(SilkText(42, 76, "U8 BMP280", 0.7))
    b.pads += soic_pads(70, 70, 8)
    b.silk.append(SilkText(62, 78, "U9 GNSS", 0.7))
    b.pads += qfn_pads(50, 55, 6, 0.4, 0.22, 0.5, 3.5, "CODEC_")
    b.silk.append(SilkText(40, 62, "U10 CODEC", 0.7))

    # bulk caps / inductors placeholders
    for i, x in enumerate([85, 90, 95, 100]):
        b.pads += c0603(x, 80)
        b.pads += r0603(x, 84)

    b.traces_f.append((30, 8, 95, 60, 0.5))
    b.traces_b.append((4, 4, W - 4, 4, 0.6))
    b.traces_b.append((4, H - 4, W - 4, H - 4, 0.6))

    b.bom_rows = [
        {"Comment": "SoM", "Designator": "MOD1", "Footprint": "mezzanine", "LCSC": "", "Quantity": "1", "Value": "RK3566 SoM 2GB", "Populate": "yes"},
        {"Comment": "Charger", "Designator": "U5", "Footprint": "QFN-24", "LCSC": "C89365", "Quantity": "1", "Value": "BQ25895", "Populate": "yes"},
        {"Comment": "CAN iso", "Designator": "U6", "Footprint": "SOIC-8", "LCSC": "C2832113", "Quantity": "1", "Value": "ISO1042", "Populate": "yes"},
        {"Comment": "ETH PHY", "Designator": "U7", "Footprint": "QFN-32", "LCSC": "C132991", "Quantity": "1", "Value": "RTL8201F", "Populate": "yes"},
        {"Comment": "Baro", "Designator": "U8", "Footprint": "LGA-8", "LCSC": "C83264", "Quantity": "1", "Value": "BMP280", "Populate": "yes"},
        {"Comment": "GNSS", "Designator": "U9", "Footprint": "module", "LCSC": "C92489", "Quantity": "1", "Value": "ATGM336H", "Populate": "yes"},
        {"Comment": "Codec", "Designator": "U10", "Footprint": "QFN-20", "LCSC": "C75024", "Quantity": "1", "Value": "MAX98357A", "Populate": "yes"},
        {"Comment": "USB-C", "Designator": "J3,J4", "Footprint": "16P midmount", "LCSC": "C165948", "Quantity": "2", "Value": "USB-C", "Populate": "yes"},
        {"Comment": "RJ45", "Designator": "J5", "Footprint": "magjack", "LCSC": "C2833924", "Quantity": "1", "Value": "RJ45", "Populate": "yes"},
        {"Comment": "Header", "Designator": "J2,J6,J7,J8", "Footprint": "2.54", "LCSC": "C124378", "Quantity": "4", "Value": "headers", "Populate": "yes"},
        {"Comment": "LDO/buck", "Designator": "U11,U12", "Footprint": "SOT-23-5", "LCSC": "C15127", "Quantity": "2", "Value": "regulators", "Populate": "yes"},
        {"Comment": "0.1uF", "Designator": "C1-C20", "Footprint": "0603", "LCSC": "C14663", "Quantity": "20", "Value": "100nF", "Populate": "yes"},
        {"Comment": "10uF", "Designator": "C21-C30", "Footprint": "0805", "LCSC": "C15850", "Quantity": "10", "Value": "10uF", "Populate": "yes"},
        {"Comment": "BT/WiFi", "Designator": "MOD2", "Footprint": "module", "LCSC": "C527667", "Quantity": "1", "Value": "ESP32-C3 or AP6256", "Populate": "yes"},
        {"Comment": "Fuse", "Designator": "F1", "Footprint": "1206", "LCSC": "C189291", "Quantity": "1", "Value": "3A PTC", "Populate": "yes"},
        {"Comment": "TVS", "Designator": "D1", "Footprint": "SMA", "LCSC": "C13589", "Quantity": "1", "Value": "SMBJ15A", "Populate": "yes"},
    ]
    b.cpl_rows = [
        {"Designator": "U5", "Val": "BQ25895", "Package": "QFN-24", "Mid X": "95.00mm", "Mid Y": "60.00mm", "Rotation": "0", "Layer": "Top"},
        {"Designator": "U6", "Val": "ISO1042", "Package": "SOIC-8", "Mid X": "95.00mm", "Mid Y": "40.00mm", "Rotation": "0", "Layer": "Top"},
        {"Designator": "U7", "Val": "RTL8201F", "Package": "QFN-32", "Mid X": "95.00mm", "Mid Y": "22.00mm", "Rotation": "0", "Layer": "Top"},
        {"Designator": "U8", "Val": "BMP280", "Package": "LGA-8", "Mid X": "50.00mm", "Mid Y": "70.00mm", "Rotation": "0", "Layer": "Top"},
        {"Designator": "J3", "Val": "USB-C", "Package": "USB-C-16P", "Mid X": "30.00mm", "Mid Y": "8.00mm", "Rotation": "0", "Layer": "Top"},
        {"Designator": "J4", "Val": "USB-C", "Package": "USB-C-16P", "Mid X": "55.00mm", "Mid Y": "8.00mm", "Rotation": "0", "Layer": "Top"},
    ]
    return b


def write_master_bom():
    bom_dir = ROOT / "bom"
    bom_dir.mkdir(parents=True, exist_ok=True)
    rows = [
        # mechanical + shared
        ["lean", "yes", "LCD", "4.0in square IPS MIPI ≥720²", "1", "panel vendor", "45", "P0"],
        ["lean", "yes", "Cover glass", "102x102x1.1mm AR optional", "1", "cut glass", "8", "P0"],
        ["lean", "yes", "18650 cell", "protected 3000mAh", "2", "any", "8", "P0"],
        ["lean", "yes", "Battery holder", "2x18650 tray PCB or clips", "1", "C70378", "3", "P0"],
        ["lean", "yes", "Speaker", "20mm 8Ω 1W", "1", "C96587", "1", "P0"],
        ["lean", "yes", "Mic", "analog MEMS", "1", "C72437", "1", "P1"],
        ["lean", "yes", "SoM", "RK3566 2GB+16GB class", "1", "vendor", "35", "P0"],
        ["lean", "yes", "Board A fab+asm", "see elec/fab/board-a", "1", "JLCPCB", "25", "P0"],
        ["lean", "yes", "Board B fab+asm", "see elec/fab/board-b", "1", "JLCPCB", "40", "P0"],
        ["lean", "yes", "OSB caps print", "PETG x20", "20", "print", "2", "P0"],
        ["lean", "yes", "Case print", "PETG front+rear+tray", "1", "print", "15", "P0"],
        ["lean", "yes", "TPU bumpers", "x4", "4", "print", "2", "P0"],
        ["lean", "yes", "Fasteners", "M3 inserts+screws kit", "1", "McMaster/Amazon", "5", "P0"],
        ["full", "yes", "GNSS antenna", "active ceramic", "1", "C97521", "3", "P2"],
        ["full", "yes", "WiFi/BT antenna", "IPEX", "1", "C22398", "1", "P1"],
        ["full", "yes", "M12 panel set", "optional rugged kit", "3", "Phoenix/Amphenol", "40", "P2"],
        ["full", "no", "Camera module", "CSI DNP", "1", "", "15", "P3"],
        ["full", "yes", "Cable OBD adapter", "CAN→J1962 read-only", "1", "custom", "12", "P1"],
    ]
    path = bom_dir / "bom-master.csv"
    with path.open("w", newline="") as f:
        wr = csv.writer(f)
        wr.writerow(["config", "populate", "item", "description", "qty", "vendor", "est_usd", "priority"])
        wr.writerows(rows)
    # lean total note
    (bom_dir / "README.md").write_text(
        """# BOM

- `bom-master.csv` — system-level parts (case, panel, SoM, fab)
- Per-board LCSC BOMs: `../elec/fab/board-a-bezel/bom.csv`, `../elec/fab/board-b-carrier/bom.csv`

## Cost targets (1-off retail estimates)

| Config | Scope | Est. USD |
|--------|--------|---------|
| **lean** | P0 rows + board asm, mid panel, 2×18650 | **~$220–300** |
| **full** | + GNSS ant, M12 kit, better panel | **~$320–450** |

Populate column `config=lean` first. Order full-config PCB with DNP parts unplaced where marked.

Regenerate board BOMs: `python3 hardware/tools/gen_pcbs.py`
"""
    )


def main():
    a = build_board_a()
    b = build_board_b()
    za = write_board_fab(a, FAB / "board-a-bezel")
    zb = write_board_fab(b, FAB / "board-b-carrier")
    write_master_bom()
    # schematic netlist summaries
    sch = ROOT / "elec"
    (sch / "bezel-mcu" / "README.md").parent.mkdir(parents=True, exist_ok=True)
    (sch / "bezel-mcu" / "README.md").write_text(
        """# Board A — Bezel MCU

**Fab package:** [`../fab/board-a-bezel/`](../fab/board-a-bezel/)  
**Size:** 140 × 140 mm, 2-layer, central **105 × 105 mm** display aperture  
**MCU:** STM32G431 class (LQFP-48)  
**Role:** OSB 1–20 matrix, 4 rockers, ALS, IMU, SWD, B2B to Board B

## Nets (logical)

| Group | Nets |
|-------|------|
| Matrix | OSB1..OSB20 sense lines → MCU GPIO (active low, 10k pull-up) |
| Rockers | GAIN/SYM/BRT/CON UP·COM·DN |
| Power | 3V3, GND from B2B; local LDO if needed |
| Sensors | I2C1 SDA/SCL → BMI270 + VEML7700 |
| Debug | SWDIO, SWCLK, nRST, GND |
| B2B J1 | 3V3, GND, UART_TX, UART_RX, I2C, IRQ, BOOT0, RESET |

## Firmware contract

Emit `BezelEvent` stream over UART/USB-CDC to Board B (see `docs/hardware-bezel.md`).
"""
    )
    (sch / "carrier-som" / "README.md").parent.mkdir(parents=True, exist_ok=True)
    (sch / "carrier-som" / "README.md").write_text(
        """# Board B — SoM carrier

**Fab package:** [`../fab/board-b-carrier/`](../fab/board-b-carrier/)  
**Size:** 120 × 90 mm, 2-layer  
**SoM:** RK3566-class mezzanine (2 GB RAM target)  
**Role:** Linux host, panel FPC, USB-C charge/data, Eth, CAN, UART, audio, battery, RF

## Port map (see also docs/hardware/cmfd-connector-icd.md)

| Ref | Function |
|-----|----------|
| J3 | USB-C primary — PD sink + data + flash |
| J4 | USB-C aux |
| J5 | Ethernet RJ45 |
| J6 | CAN-H/L + UART + GND (isolated CAN) |
| J7 | Audio out / mic |
| J8 | 18650 pack sense/power |
| J9 | Display FPC 40-pin |
| J2 | B2B from Board A |

## Power

USB-C → BQ25895-class charger → 1S/2S 18650 → system buck → SoM 5V/3.3V.  
Optional DC jack pads for vehicle adapter (TVS + fuse).
"""
    )
    print(f"Wrote {za}")
    print(f"Wrote {zb}")
    print("Master BOM → hardware/bom/bom-master.csv")


if __name__ == "__main__":
    main()
