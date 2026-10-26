# AGENTS.md — hosts/

One directory per machine; only parker exists (giskard, the MacBook, keeps
its own dotfiles repo — deliberate, see DECISIONS.md).

`parker/default.nix` composes system/ modules and sets identity
(hostname, stateVersion) — it never configures.
`parker/disko.nix` owns the OS disk (Samsung 980) declaratively; the three
ext4 data drives are plain mounts in `parker/hardware.nix`, untouched by any
install. `hardware.nix` was pre-baked without machine access — install day
diffs it against `nixos-generate-config --no-filesystems` output.

stateVersion (system and home): "26.05", the release current at first
install. Never bump it on updates.
