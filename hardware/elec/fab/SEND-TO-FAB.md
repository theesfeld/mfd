# Send these files to a PCB house

**Issue:** [#137](https://github.com/theesfeld/mfd/issues/137)

## Board A — bezel MCU

1. Upload zip: `cmfd-board-a-bezel-gerbers.zip`  
   (or all files in `board-a-bezel/` except README if the house wants loose Gerbers)
2. Options: **2 layers**, **1.6 mm**, **HASL lead-free** or ENIG, min track/space **6/6 mil** OK
3. Qty: 5 pcs recommended for first spin
4. SMT assembly (optional): `board-a-bezel/bom.csv` + `board-a-bezel/cpl.csv`

## Board B — SoM carrier

1. Upload zip: `cmfd-board-b-carrier-gerbers.zip`
2. Same stack-up as Board A
3. SMT assembly: `board-b-carrier/bom.csv` + `board-b-carrier/cpl.csv`
4. **SoM is not on the SMT BOM** — order the RK3566-class module separately and seat on mezzanine

## Important (read before $)

These Gerbers are **generated from the project design script** (`hardware/tools/gen_pcbs.py`) for a **first-article purpose-built layout**: outline, pad lands, drill map, silk labels, BOM/CPL.

Before **paid full assembly** of dense ICs:

1. Open Gerbers in [gerber-viewer](https://www.gerber-viewer.com/) or KiCad Gerber viewer.  
2. Confirm outline, aperture, and drill registration.  
3. For production density (STM32 pin escape, USB-C CC, SoM high-speed), plan a **KiCad refinement pass** on the same mechanical envelope — the generator freezes geometry and part intent.

Bare-board fab of this package is appropriate for **fit-check** (case, OSB holes, standoffs, port windows).

## Also order

| Item | Source |
|------|--------|
| Case STLs | `hardware/mech/print/cmfd-print-files.zip` |
| 4″ square IPS panel | vendor shortlist in BOM |
| SoM module | RK3566 class 2 GB |
| 18650 cells (protected) | separate — do not put Li-ion in PCB carton |
| Switches 6×6 and rockers | LCSC / Digi-Key per board BOM |
