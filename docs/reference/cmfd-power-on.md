# Real F-16 CMFD power-on (public / training model)

**Issue:** [#88](https://github.com/theesfeld/mfd/issues/88)  
**SoT:** MLU M1 Pilot’s Guide CMFD controls + Format Select (OSB 12/13/14); public DCS F-16 cold-start behavior.

## What the glass actually shows

| Phase | What pilots see | What we draw |
|-------|-----------------|--------------|
| **1. Power apply** | Dark / blank LCD | Full black face (no OSB text) |
| **2. Display alive** | **BLANK** or **DTC-loaded default format** on the face | MLU bezel OSB chrome + **empty** content (true blank — no center “BLANK” word) |
| **3. Formats ready** | Assigned formats (e.g. FCR / HSD / SMS on OSB 14/13/12) | Bottom: `SWAP` · `FCR` · `HSD` · `SMS` · `DCLT` (training default left CMFD) |
| **4. Mission glass** | Sensor formats with video/symbology | For **this product:** vehicle systems pages after probe |

### What we **do not** invent

- No progress-bar “loading” splash  
- No multi-line MFDS/PCM GO checklist as the **power-on face** (that is closer to a selectable **TEST/BIT** format, not cold power)  
- No marketing logo or version splash in the content area  

Capability probe (OBD/UDS options) still runs **off-glass** during phases 1–3. When probe is ready, the product switches to vehicle systems pages.

### OSB layout (MLU Figure 1-14, power-on default)

```
        GAIN                              SYM
     [1] [2] [3] [4] [5]     (empty on blank face)
 [20]                      [6]
 [19]                      [7]
 [18]      [ black ]       [8]
 [17]                      [9]
 [16]                      [10]
        BRT               CON
     [15][14][13][12][11]
      SWAP FCR  HSD  SMS DCLT
```

Active format slot (OSB 14) is highlighted when the display is alive.

### Vehicle product note

This hardware CMFD reuses **real power-on look**, then hands off to auto systems pages. Jet FCR/SMS symbology is **not** required after phase 3 for the truck product.
