/**
 * Single source of product geometry for the exploded viewer.
 * Values mirrored from:
 *   - hardware/tools/kicad_build.py  (Board A/B placement)
 *   - hardware/mech/src/cmfd_enclosure.scad
 *   - hardware/ORDER-THIS.md BOM
 * Units: millimetres.
 */
export const CMFD = {
  issue: 137,
  // Enclosure (OpenSCAD)
  outer_xy: 148,
  outer_z: 58,
  wall: 3.2,
  corner_r: 8,
  glass_xy: 102,
  bezel_front_z: 14,
  rear_z: 44, // outer_z - bezel_front_z
  // Board A — passive switch panel (KiCad)
  boardA: {
    w: 148,
    h: 148,
    thickness: 1.6,
    cut: 102,
    cut_x: 23, // (148-102)/2
    cut_y: 23,
    holes: [
      [16, 16],
      [132, 16],
      [132, 132],
      [16, 132],
    ],
    gnd_header: [74, 20], // W/2, 20
    // 6x6 mm tactile SW_PUSH_6mm_H4.3mm
    switch_xy: 6.0,
    switch_z: 4.3,
    // pin header 1x01
    pin_pitch: 2.54,
    pin_post_z: 8.0,
  },
  // Board B — carrier
  boardB: {
    w: 120,
    h: 90,
    thickness: 1.6,
    holes: [
      [8, 8],
      [112, 8],
      [112, 82],
      [8, 82],
    ],
    j_bezel: [55, 18],
    j_rk: [100, 18],
    j_gnd: [110, 18],
    som_a: [30, 45],
    som_b: [30, 62],
    usbc: [75, 50],
    rj45: [95, 70],
    reg: [70, 72],
    // approximate component sizes (BOM-class)
    som_xy: [45, 32, 3.5],
    usbc_xy: [9, 7, 3.2],
    rj45_xy: [16, 21, 13],
  },
  // OSB layout (KiCad osb_layout)
  osb_inset: 14,
  osb_span: 78,
  // Rockers (KiCad)
  rockers: {
    GAIN: [26, 148 - 26],
    SYM: [148 - 26, 148 - 26],
    BRT: [26, 26],
    CON: [148 - 26, 26],
  },
  // BOM / physical parts
  bom: {
    lcd: { xy: 102, z: 3.2, active: 98 }, // 4" class IPS package
    cell_18650: { d: 18.2, h: 65 },
    battery_tray: { w: 42, h: 78, z: 22 },
    osb_cap: { xy: 12, z: 6 },
    rocker_cap: { w: 14, h: 10, z: 5 },
    bumper: { xy: 22, z: 54 },
    wire_awg: 0.6,
  },
  // Aesthetic (ORDER-THIS + industrial design)
  colors: {
    bezel_silver: 0xb8c6d6,
    walkman_blue: 0x2a6db5,
    rear_grey: 0x6a7585,
    fr4: 0x1b4d2e,
    fr4_mask: 0x0d3b24,
    copper: 0xb87333,
    gold: 0xd4af37,
    silk: 0xe8eef5,
    plastic_cap: 0xe8ecf0,
    tpu: 0x2a2a2a,
    lcd_black: 0x05080a,
    lcd_glass: 0x88aacc,
    pin_metal: 0xc0c5cc,
    switch_body: 0x1a1a1a,
  },
};

/** OSB 1..20 centers [x,y] matching KiCad clockwise from top-left. */
export function osbCenters(W = CMFD.boardA.w, H = CMFD.boardA.h) {
  const inset = CMFD.osb_inset;
  const span = CMFD.osb_span;
  const step = span / 4;
  const c = W / 2;
  const out = [];
  for (let i = 0; i < 5; i++) out.push({ id: 1 + i, x: c - span / 2 + i * step, y: H - inset, side: "TOP" });
  for (let i = 0; i < 5; i++) out.push({ id: 6 + i, x: W - inset, y: c + span / 2 - i * step, side: "RIGHT" });
  for (let i = 0; i < 5; i++) out.push({ id: 15 - i, x: c - span / 2 + i * step, y: inset, side: "BOT" });
  for (let i = 0; i < 5; i++) out.push({ id: 20 - i, x: inset, y: c + span / 2 - i * step, side: "LEFT" });
  return out;
}

/** Pin header offset toward outer edge (KiCad pin_offset). */
export function pinOffset(side, x, y, dist = 5.5) {
  let hx = x,
    hy = y;
  if (side === "TOP") hy = y + dist;
  else if (side === "BOT") hy = y - dist;
  else if (side === "LEFT") hx = x - dist;
  else hx = x + dist;
  const W = CMFD.boardA.w,
    H = CMFD.boardA.h;
  hx = Math.min(Math.max(hx, 3.5), W - 3.5);
  hy = Math.min(Math.max(hy, 3.5), H - 3.5);
  return { x: hx, y: hy };
}
