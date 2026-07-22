# BOM

- `bom-master.csv` — system-level parts (case, panel, SoM, fab)
- Per-board LCSC BOMs: `../elec/fab/board-a-bezel/bom.csv`, `../elec/fab/board-b-carrier/bom.csv`

## Cost targets (1-off retail estimates)

| Config | Scope | Est. USD |
|--------|--------|---------|
| **lean** | P0 rows + board asm, mid panel, 2×18650 | **~$220–300** |
| **full** | + GNSS ant, M12 kit, better panel | **~$320–450** |

Populate column `config=lean` first. Order full-config PCB with DNP parts unplaced where marked.

Regenerate board BOMs: `python3 hardware/tools/gen_pcbs.py`
