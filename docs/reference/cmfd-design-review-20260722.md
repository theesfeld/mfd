# CMFD design board minutes — 2026-07-22

**Roles:** glass design · HMI · electrical  
**Product:** vehicle CMFD (`cmfd`), display-only OBD  
**SoT:** MLU M1 Fig 1-14/1-15 · Table 1-1 · `vehicle-cmfd-design.md`

## Verdict

Keep MLU control skeleton (20 OSB, slots 12–14, Master Menu, blank-not-repack).  
Vehicle deltas (OWN, support DTC/SET/BUS, ENG/DRV/ATT) are correct.

## Implemented

| Wave | Content | Issue / PR |
|------|---------|------------|
| P0 | Support return; lab OsbUp; lab chrome gate; no RNG invent; LITE honesty | #133 / #134 |
| P1/P2 | Green SOI box; empty Master Menu; gauge palette; SYM/CON; ATT sky/ground; short header; DCLT label | #135 |

## Lab keys (not production muscle memory)

`1234567890qwertyuiop` = OSB 1–20 · `[` `]` prev/next · `-` `=` BRT · `;` `'` CON · `\` `|` SYM

## Remaining (electrical / later)

- Connector pinout + rocker BOM freeze  
- `GpioBezel` with Down+Up  
- OSB-only acceptance harness (ban lab aliases)  
