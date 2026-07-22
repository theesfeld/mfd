# MFD photo and page-type reference index

**Purpose:** Public references for layout/type research.  
**Not** a dump of copyrighted image binaries into the repo.  
**Not** classified OEM symbology.

Use these to learn **page types**, **chrome**, **color**, and **widget density**. Prefer USAF public affairs, museums, manufacturer public pages, and open flight-sim documentation.

## How to use this list

1. Open the URL.  
2. Note page type (SMS, HSD, TGP, …).  
3. Note widgets (softkeys, tapes, rings, gates, text).  
4. Implement as `mfd::jet::*` or a new widget — do not copy proprietary art.

## Fighter MFD page types (catalog)

| Code | Name | Primary widgets | Library call |
|------|------|-----------------|--------------|
| BLANK | Blank | Softkeys | `jet::blank` |
| SMS | Stores management | Softkeys, station grid, arm flag | `jet::sms` |
| HSD | Horizontal situation | Softkeys, range rings, heading | `jet::hsd` |
| TGP | Targeting pod | Softkeys, FOV, track gate | `jet::tgp` |
| FCR | Fire-control radar | Softkeys, B-scope grid, contact | `jet::fcr` |
| ENG | Engine | Round gauges, tapes | `jet::eng` |
| FUEL | Fuel | Tapes | `jet::fuel` |
| DTE | Data transfer | Softkeys, list | `jet::dte` |
| TEST | Built-in test | Softkeys, status | `jet::test` |
| WPN | Weapons | Softkeys, lists, cues | *(extend)* |
| HAD | HAS/HAD | Softkeys, targeting | *(extend)* |
| FLIR | FLIR video + symbology | Softkeys, gate, FOV | *(extend)* |
| CNI | Comm/nav/ident | Softkeys, list | *(extend)* |
| UFC | Up-front control mirror | Labels, keys | *(extend)* |
| DED | Data entry display | Text rows | *(extend)* |
| PFL | Pilot fault list | List, caution color | *(extend)* |
| RESET | Reset/status | Labels | *(extend)* |

## Public reference links (50+)

> Counts and hosts change. These are **starting points** for high-resolution public imagery and technical pages. Search the same titles on official domains if a link moves.

### USAF / government / museum (public)

1. https://www.af.mil/ — search “F-16 cockpit” / “MFD”  
2. https://www.navy.mil/ — search “F/A-18 cockpit display”  
3. https://www.nationalmuseum.af.mil/  
4. https://www.nasa.gov/ — research cockpit / HUD imagery  
5. https://media.defense.gov/ — public defense imagery search  
6. https://www.dvidshub.net/ — search F-16 cockpit MFD  
7. https://commons.wikimedia.org/wiki/Category:F-16_Fighting_Falcon_cockpits  
8. https://commons.wikimedia.org/wiki/Category:McDonnell_Douglas_F/A-18_Hornet_cockpits  
9. https://commons.wikimedia.org/wiki/Category:Head-up_display  
10. https://commons.wikimedia.org/wiki/Category:Glass_cockpits  

### Technical / design references

11. https://wiki.flightgear.org/Howto:Getting_started_with_Glass_Cockpit_Avionics_Development  
12. https://wiki.flightgear.org/Canvas_glass_cockpit_efforts  
13. https://wiki.flightgear.org/Canvas_ND_framework  
14. https://opengc.sourceforge.net/  
15. https://github.com/gtraines/OpenMFD  
16. https://openhornet.com/  
17. https://github.com/jrsteensen/OpenHornet  
18. https://makerplane.org/  
19. https://avionicsduino.com/index.php/en/  
20. https://b612-font.com/ (font design brief; not photos)

### F-16 page types (search phrases for high-res stills)

Use image search / DVIDS / commons with exact phrases:

21. `F-16 SMS MFD`  
22. `F-16 HSD MFD`  
23. `F-16 TGP MFD`  
24. `F-16 FCR RWS MFD`  
25. `F-16 DTE MFD`  
26. `F-16 WPN MFD`  
27. `F-16 HAD MFD`  
28. `F-16 cockpit MFD OSB`  
29. `F-16 ICP DED display`  
30. `F-16 engine page MFD`  

### F/A-18 / multi-type (search phrases)

31. `F/A-18 SA page MFD`  
32. `F/A-18 stores page`  
33. `F/A-18 radar attack format`  
34. `F/A-18 FLIR MFD`  
35. `F/A-18 HSI MFD`  
36. `F-15 MFD cockpit`  
37. `F-22 cockpit display` (public airshow only)  
38. `F-35 cockpit panoramic` (public/manufacturer only)  
39. `A-10 CMFD`  
40. `Eurofighter cockpit MFD`  

### Color / symbology notes (public discussion)

41. https://aviation.stackexchange.com/questions/17714/why-are-huds-usually-green-instead-of-red  
42. Search: `avionics display green cyan amber red symbology`  
43. Search: `NVG compatible cockpit display colors`  
44. Search: `MIL-STD-1787 HUD symbology` (public summaries)  
45. Search: `ARINC 661` (CDS architecture; commercial glass)

### Sim documentation (layout only — do not ship assets)

46. DCS F-16C Early Access Guide (Eagle Dynamics public manual pages)  
47. DCS F/A-18C Early Access Guide  
48. BMS Falcon documentation (community)  
49. FlightGear F-16 / aircraft Canvas MFD sources  
50. X-Plane glass cockpit tutorials  

### Additional public still sources

51. https://www.flickr.com/search/?text=F-16%20cockpit%20MFD (filter commercial use as needed)  
52. Airshow official media galleries (search host nation air force)  
53. Manufacturer public media (Lockheed Martin, Boeing media rooms)  
54. https://www.airforce-technology.com/ (articles with cockpit photos)  
55. https://theaviationist.com/ (public articles; respect copyright)  

## Fighter color tokens (library)

| Role | Typical ink | `mfd::color` |
|------|-------------|--------------|
| Normal / mode | Green | `GREEN` |
| Structure / dim | Dim green | `GREEN_DIM` |
| Geometry / nav | Cyan | `CYAN` |
| Caution | Amber | `AMBER` |
| Warning / limit | Red | `RED` |
| Primary readout | White | `WHITE` |
| Special cue | Magenta | `MAGENTA` |
| Glass | Black | `BLACK` |

## Aviation → automotive reuse

| Jet call / widget | Auto use |
|-------------------|----------|
| `tape_gauge` / `jet::fuel` | Fuel, coolant, oil, trans temp |
| `round_gauge` / `jet::eng` | Tachometer (RPM), boost |
| Softkeys | Drive mode / page select |
| Labels / DTE list | OBD PID list |
| `auto::cluster` | Driver cluster page |
| `auto::obd_status` | Mode 01 PID snapshot (stub) |

---

*Expand this index with stable archive.org or DVIDS asset IDs when you pin a specific still for design review.*
