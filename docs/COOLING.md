# COOLING.md — fans, curves, and what the hardware actually does

Diagnosed and tuned 11 Oct 2026 (coolercontrold 5.0.1). The curves live in
CoolerControl's own state, `/etc/coolercontrol/config.toml`, which nix does
not manage; `docs/cooling/coolercontrol-curves.toml` is the reviewable
snapshot and the restore source after a reinstall.

## Fan map

Identified one channel at a time (stop it for 5 s, Peter watches the case).

| CoolerControl channel | Physical fan(s) | Notes |
|---|---|---|
| Kraken 2023 · `fan` | 3× top radiator F140 RGB Core, exhausting up through the rad | One tach for all three. Will not stop at 0% (reports 0 rpm, keeps spinning) |
| Kraken 2023 · `pump` | pump | ~1550 rpm at 35%, ~3050 at 100% |
| nct6798 · `fan1` | front middle intake | white NZXT, LEDs unpowered (docs/LIGHTING.md) |
| nct6798 · `fan6` | front top intake | ″ |
| nct6798 · `fan7` | front bottom intake | ″ |
| nct6798 · `fan5` | rear exhaust | ″ |
| nct6798 · `fan3` | **not a fan** — the Kraken's pump tach lead | Tracks pump rpm (1554→1477, 2010→1950), ignores PWM. Leave it alone |
| nct6798 · `fan2`, `fan4` | empty headers | fan2 is likely CPU_FAN, satisfied by nothing |

Airflow is 3 intakes behind a glass front against 4 exhausts: negative
pressure in a starved case, so the intakes run a few points above the
exhaust. The radiator breathes GPU-warmed case air; that is the price of
top-mount exhaust, which is still the right orientation.

## Curves

Everything keys on **Kraken liquid temperature**, never CPU temperature: the
14700KF hits 100 °C within seconds of any heavy load whatever the fans do,
so a CPU-keyed curve would roar on every shader compile for nothing. Liquid
is slow and is the quantity the radiator actually has to shed. Case fans
take the **max** of a liquid curve and a GPU curve, because the 3090 dumps
350–420 W into the case while gaming.

| Profile | Drives | Source | Points (°C → %) |
|---|---|---|---|
| Radiator | Kraken fan | liquid | ≤36 → 35 · 39 → 55 · 43 → 100 |
| Pump | Kraken pump | liquid | ≤37 → 55 · 40 → 80 · 43 → 100 |
| Front intakes | fan1/6/7 | max(liquid, GPU) | liquid ≤37 → 40 · 41 → 60 · 45 → 85; GPU ≤55 → 40 · 75 → 85 |
| Rear exhaust | fan5 | max(liquid, GPU) | liquid ≤37 → 35 · 41 → 55 · 45 → 80; GPU ≤55 → 35 · 75 → 80 |

Graph profiles use the `Smooth` function (2 °C deviance, 3 s response delay,
hysteresis on the way down only). Idle liquid sits at **36–37 °C** here, not
the ~30 one might assume — curves that start ramping at 30 spin up at idle.
The GPU's own fans stay on its vbios curve (0 rpm idle); CoolerControl only
reads its temperature.

## Measurements (stress-ng, all 28 threads, 4 min, ~22 °C room)

| | Kraken firmware curve (before) | Kraken at 100% | These curves |
|---|---|---|---|
| Radiator fans | stuck ≤60%, ~1100 rpm | ~1760 rpm | 54% → 100% within ~75 s |
| Liquid at 4 min | 51 °C and climbing | 43 °C, flat | 44 °C, flat |
| First ~130 s (~240–258 W) | 100 °C, throttling | 100 °C, throttling | 100 °C, throttling |
| After ~130 s (~205 W) | 100 °C, throttling | 91 °C, none | 91 °C, none |
| All-core average clock | 4.5–4.7 GHz | 4.6–4.8 GHz | 4.6–4.8 GHz |

Reading it:
- **The cooler is healthy.** At 205 W the cores sit ~48 °C over the liquid
  (≈0.23 °C/W) with an 8–15 °C spread across P-cores — normal contact, no
  mount or paste problem.
- **Bursts at ~250 W hit TjMax regardless of airflow.** That is Raptor Lake
  at that power, not a fault.
- **Sustained load is where the curve matters:** at ~205 W, 43 °C liquid is
  91 °C and full clocks; 51 °C liquid is 100 °C and throttling.
- The step from ~240 W to 205 W at ~130 s looks like a longer-window BIOS
  limit behind the PL1 = PL2 = 265 W that RAPL reports. Unconfirmed — the
  MSRs need root.

## Changing curves

The UI works, and so does the REST API (`https://127.0.0.1:11987`,
self-signed, bearer token from PARKER.md). Shapes verified on 5.0.1:

```sh
H=(-sk -H "Authorization: Bearer $TOKEN" -H 'Content-Type: application/json')
curl "${H[@]}" -X PUT  $A/profiles -d '{"uid":"radiator", "p_type":"Graph", "speed_profile":[[20,30],[43,100]], "temp_source":{"temp_name":"liquid","device_uid":"<kraken>"}, "function_uid":"parker-smooth", "name":"…"}'
curl "${H[@]}" -X PUT  $A/devices/<uid>/settings/<channel>/profile -d '{"profile_uid":"radiator"}'
curl "${H[@]}" -X PUT  $A/devices/<uid>/settings/<channel>/manual  -d '{"speed_fixed":40}'
```

`POST /profiles` and `POST /functions` create; `PUT` updates. Channel
labels are UI-only: `PUT …/settings/<channel>/label` returns 405.
`…/settings/<channel>/reset` returns 200 but leaves a Kraken channel at its
last duty — it does not hand control back to the firmware curve.

## Open

- **PL1** in the BIOS: 265 W against Intel's 125 W base / 253 W turbo for
  the 14700KF. Dropping ~35 W cost ~3% all-core clock and ~9 °C in the test
  above. Not urgent with the curves in place.
- **BIOS** is 4.12 (Sep 2023) and ships microcode 0x11d, which predates the
  Raptor Lake Vmin-shift fix (0x12B). The kernel log shows NixOS updating
  it early to 0x137 (`hardware.cpu.intel.updateMicrocode`), so the OS runs
  fixed; firmware and bootloader still run on 0x11d every boot. A BIOS
  update closes that gap — worth doing, not urgent.
- **GPU under real load** is unmeasured. Log it during a gaming session.
