# RUNBOOK.md — install day, step by step

Follow top to bottom. Commands are copy-pasteable. Read from the MacBook;
after Part 2 step 4 you can SSH in and stop touching parker's keyboard.

Context: the graphical ISO's installer goes **unused** — we only want its
live environment (it has NetworkManager for wifi). The repo is the install.

---

## Part 0 — before leaving the MacBook

- [ ] CI is green on `main` (the repo builds).
- [ ] Backup exists: `/mnt/hdd1/parker-backup/` (ssh keys, Slippi,
      coolercontrol config, hardware captures). Already verified, re-verify
      if paranoid — after disko runs, the old OS disk is gone.
- [ ] USB stick flashed with the graphical NixOS ISO.

## Part 1 — live USB → installed system (~30 min, mostly downloads)

**1. Boot the USB.** Spam the BIOS boot-menu key at power-on, pick the USB.
You'll land in a GNOME live session. Ignore the graphical installer
completely. Open a terminal (Console).

**2. Become root.** Everything below assumes a root shell:

```bash
sudo -i
```

**3. Wifi:**

```bash
nmtui
```

→ *Activate a connection* → pick the network → enter the password. Then
verify:

```bash
ping -c 3 github.com
```

**4. Get git and clone the repo** (git isn't guaranteed in the live ISO):

```bash
nix-shell -p git
```

```bash
git clone https://github.com/Cabbagito/parker.git /root/parker && cd /root/parker
```

**5. Confirm the target disk exists under its by-id name** (paranoia — disko
only addresses this exact id, the data drives are structurally untouchable):

```bash
ls -l /dev/disk/by-id/ | grep -i samsung
```

Expect `nvme-Samsung_SSD_980_500GB_S64DNL0W226998W`. If it's absent, STOP —
do not improvise device names; figure out why first.

**6. Hardware sanity diff** (~60s). The repo's `hosts/parker/hardware.nix`
was pre-baked blind; compare against what the machine actually reports:

```bash
nixos-generate-config --no-filesystems --show-hardware-config > /tmp/hw.nix; diff /tmp/hw.nix hosts/parker/hardware.nix
```

The files are *structurally* different (the generated one has imports and
boilerplate) — you are only checking one thing: does the generated
`boot.initrd.availableKernelModules` list contain modules ours doesn't
(ours: nvme, xhci_pci, ahci, usbhid, usb_storage, sd_mod)? If yes, add them
to `hosts/parker/hardware.nix` with nano here in the live clone (the install
uses this local clone), and re-make the same edit + push from parker after
first boot.

**7. Partition + format + mount — THE destructive step.** Wipes the Samsung
980 (only). It asks for confirmation before destroying:

```bash
nix --experimental-features "nix-command flakes" run github:nix-community/disko/latest -- --mode destroy,format,mount --flake .#parker
```

Verify the layout landed:

```bash
findmnt --real | grep /mnt
```

Expect: `/mnt` (btrfs), `/mnt/boot` (vfat), `/mnt/home`, `/mnt/nix`,
`/mnt/var/log`. (The ext4 data drives are *not* mounted here — correct;
they belong to the running system, not the installer.)

**8. Install.** Root stays locked (`--no-root-passwd`); your user comes
from the config, its sudo password does not (DECISIONS.md, since October
2026):

```bash
nixos-install --no-root-passwd --option experimental-features "nix-command flakes" --flake .#parker
```

Wifi will make this the long step. It ends with `installation finished!`.
Then give peki a password, or sudo has nothing to check:

```bash
nixos-enter --root /mnt -- passwd peki
```

The other machine-held secrets need nothing here: the engine password and
the SearXNG key mint themselves on first activation, and the CoolerControl
token is minted in its UI after boot and dropped into
`~/.local/state/parker/coolercontrol.token` (badge.nix).

**9. Thirty-second sanity check before rebooting:**

```bash
nixos-enter --root /mnt -- sh -c 'id peki && ls /home && ls /boot/EFI'
```

Expect: peki's uid/groups (wheel, networkmanager, docker), a `peki` home
dir, and EFI boot entries. If any is missing, debug now — this shell is
cheaper than a boot loop.

**10. Reboot:**

```bash
reboot
```

Pull the USB when the screen goes dark.

## Part 2 — first boot (~15 min)

**1. You land logged-in.** systemd-boot → TTY, already `peki@parker` with a
zsh prompt (p10k looks rough on a raw TTY — cosmetic, ignore).

**2. Wifi, once and forever:**

```bash
nmtui
```

Same dance. NetworkManager persists it; every future boot reconnects alone.

**3. Grab the IP** (no mDNS yet, so SSH by IP for now):

```bash
ip -4 addr show wlan0
```

**4. From the MacBook** — everything after this can happen over SSH:

```bash
ssh peki@<that-ip>
```

(giskard's key is baked into the config — no password prompt expected.)

**5. Restore identity from the backup** (data drives are already mounted):

```bash
ls /mnt/hdd1/parker-backup/
```

Copy the ssh keys back (adjust the source path to what's actually there):

```bash
cp -r /mnt/hdd1/parker-backup/.ssh ~/.ssh && chmod 700 ~/.ssh && chmod 600 ~/.ssh/*
```

Sanity: `ssh -T git@github.com` → "Hi Cabbagito!".
Slippi dirs wait until the gaming layer exists.

**6. Clone the repo onto parker — its permanent home:**

```bash
git clone git@github.com:Cabbagito/parker.git ~/parker
```

**7. Prove self-hosting** (build without switching — result should be
near-instant and boring, since the running system *is* this config):

```bash
cd ~/parker && sudo nixos-rebuild build --flake .#parker
```

**8. Auth Claude Code:**

```bash
claude
```

Headless login: it prints a URL — open it on the MacBook, paste the code
back into the TTY/SSH session.

**Done.** The machine is reproducible, remote-accessible, self-rebuilding,
and Claude-authed. Phase 4 (INSTALL.md) starts from here — desktop built
decision by decision, every step one rollback away.

## If something goes wrong

- **A rebuild breaks later:** reboot → systemd-boot menu → pick the previous
  generation. That's the safety net for all of Phase 4.
- **No wifi device in the live env or after boot** (`ip link` shows no
  `wlan0`): firmware issue — in the live env check `dmesg | grep iwlwifi`.
  Our config sets `hardware.enableRedistributableFirmware`, the live ISO
  ships firmware already; a missing wlan0 means something deeper, stop and
  investigate from the MacBook.
- **disko refuses / errors mid-way:** safe to re-run (destroy,format,mount
  is idempotent from a wiped state). Nothing on the data drives is at risk.
- **Boot hangs after install:** likely initrd can't find the disk → boot the
  USB again and re-check step 6 (hardware diff) — a missing
  `availableKernelModules` entry is the classic cause.
- **Matches an old symptom?** Check QUIRKS.md before inventing new fixes.
