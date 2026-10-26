# LIGHTING — every glowing thing in parker, and how to drive it

Capability reference for the case's light surfaces. Everything here was
wire-verified on parker (July 2026) unless marked otherwise. The design
record for *why* lives in PARKER.md; the theme integration lives in
`home/theming/halo.nix` (the `halo` command) and `home/parker/badge.nix`
(the pump LCD loop).

## The one-writer rule

One tool owns each device, permanently. Two writers on one controller
fight, and some of these buses (SMBus) must never carry concurrent RGB
traffic from two programs.

| Surface | Owner | Nothing else touches it |
|---|---|---|
| Pump LCD | CoolerControl (badge service streams through its API) | liquidctl would collide with CC on hidraw |
| Fan rings ×3 | liquidctl | OpenRGB also sees it (as "Hue 2") — never use both |
| GPU, RAM, mobo | OpenRGB | one process at a time; sequential invocations are fine |

## The surfaces

### Pump LCD — NZXT Kraken 2023 (base, `1e71:300e`)

240×240 @ 30 Hz behind circular glass. Composition rule: stay inside
r≈112 of centre or the circle illusion dies. Driven via CoolerControl's
REST API (`PUT /devices/{uid}/settings/lcd/lcd/images?log=false`,
Bearer `cc_` token, multipart, one image per request). Measured ~0.2 s
per applied frame; 3–4 fps streaming is sustainable indefinitely and is
how the badge loops — **firmware 2.x rejects animated GIFs** (liquidctl
#631), so the device can never loop unaided. Static images persist and
reapply on boot. Bonus endpoint: `lcd/shutdown-image` (a machine-is-off
face, unused so far).

### Fan rings — NZXT 2023 RGB Controller (`1e71:2012`)

3 channels (`led1..led3` = the three top F140 RGB Core fans, right/
middle/left as seen from the side panel), 8 addressable LEDs per ring.
CoolerControl detects this controller and *declines* it, so liquidctl
writes conflict with nothing. Effects run **on the controller** —
fire-and-forget, but forgotten on power loss (the sync timer reasserts).

```sh
liquidctl -m "2023 RGB" set led1 color fixed C1121F        # solid
liquidctl -m "2023 RGB" set led1 color super-fixed <c1>...<c8>  # per-LED ring gradient
liquidctl -m "2023 RGB" set led1 color breathing C1121F    # on-device animation
# other modes: fading, pulse, marquee-*, covering-marquee,
# alternating-*, moving-alternating-*, spectrum-wave, candle, ...
```

### GPU — MSI RTX 3090 Suprim X (ITE controller on NVIDIA I2C)

**One logical zone** (the three "Colors" are effect palette slots, not
zones) → no spatial gradients. Hardware modes run on the card:
`direct` (= its static mode), `breathing`, `flashing`, `magic`,
`patrolling`, `rainbow`, `off`, … in our colour. Trap: settings are
volatile until a one-time **"Save To Device"** in `openrgb --gui` (the
CLI cannot trigger the hardware save); until then a power cycle reverts
to factory rainbow and the sync timer corrects at login.

```sh
openrgb --noautoconnect -d Suprim -m direct -c C1121F
openrgb --noautoconnect -d Suprim -m breathing -c C1121F
```

### RAM — 2× Corsair Vengeance RGB DDR5 (CMH32GX5M2B6000C30)

10 LEDs per stick, RGB controller at SMBus 0x19/0x1B — safely distinct
from the SPD EEPROMs at 0x51/0x53, which the kernel `spd5118` driver
guards (keep it loaded; CoolerControl's DIMM temps depend on it, and it
*blocks* the dangerous addresses rather than conflicting). `custom` mode
and all named effects save **on-stick** and survive power-off. `direct`
is volatile software streaming — demo-only. Applying anything replaces
the factory rainbow permanently (one-way; `rainbow wave` approximates
it).

Two traps, both cost us a white-RAM afternoon:
1. `-m custom -c <hex>` in ONE command writes **white**, not the colour.
   Mode first, colour second, separate invocations.
2. Combining the sticks with other devices in one multi-`-d` command
   mis-associates modes across blocks.

```sh
openrgb --noautoconnect -d Vengeance -m custom      # 1: mode
openrgb --noautoconnect -d Vengeance -c C1121F      # 2: colour (all 10 LEDs)
openrgb --noautoconnect -d Vengeance -c c1,c2,...,c10   # vertical gradient, saved on-stick
openrgb --noautoconnect -d Vengeance -m "color pulse" -c C1121F,3A0508  # on-stick breathe
```

### Mobo / case strip — ASRock B760 Pro RS WiFi Polychrome (`26ce:01a2`)

Zones: 2 RGB headers, 2 addressable headers (the case glow strip lives
here), 5 PCB LEDs. Hardware modes stored on the controller: `static`,
`breathing`, `strobe`, `water`, `spring`, `rainbow`, … CLI addresses the
whole device; per-zone control needs the GUI or the OpenRGB SDK.

```sh
openrgb --noautoconnect -d B760 -m static -c C1121F
openrgb --noautoconnect -d B760 -m breathing -c C1121F
```

### Not controllable (hardware gaps)

Front 3 white NZXT fans and the exhaust fan: no controller connection —
their LEDs (if any) are unpowered until an NZXT extension/controller is
bought. RAM/GPU/fans cover everything currently wired.

## Access

All non-root: liquidctl and openrgb udev rules (`system/cooling.nix`)
tag the NZXT controllers, the Polychrome, and `/dev/i2c-*` with
`uaccess`, so the logged-in seat gets ACLs. Caveat: a system service
running outside the seat would need root or an `i2c` group rule — moot
while everything runs as user units / user commands.

## Sync: what's possible, honestly

- **Colour sync** (everything the same palette): solved — `halo sync`,
  driven by the theme, reasserted hourly and at login.
- **Tempo sync** (everything breathing at a similar rate): possible with
  on-device effects, but approximate — each controller free-runs its own
  clock, so phases drift. Reads as "the case is breathing", not as a
  metronome.
- **Frame-exact sync** (fans flash the moment the badge's core beats):
  only by software-streaming every device simultaneously, which is
  fine for *moments* (an alert sequence, a celebration sweep — one-shot
  choreography from a script or parkerd) but wrong for the permanent
  resting state (SMBus streaming flickers; volatile; fragile).

The design consequence: resting state = on-device effects + halo
colours (robust, survives everything); choreography = short streamed
sequences conducted by parkerd when something is worth saying.
