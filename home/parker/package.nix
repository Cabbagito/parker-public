# The `parker` binary. Built from home/parker/parkerd/ by nixpkgs' rust
# platform — no crane, no rust-overlay, no extra flake input; nixpkgs ships a
# current toolchain and `cargoLock.lockFile` means there is no cargoHash to
# recompute, ever.
#
# The fileset is not decoration: anything inside parkerd/ that is not a Cargo
# input (a README, a stray note) would otherwise be part of the derivation's
# source hash, and touching it would rebuild all 102 dependency crates. Keep
# .nix files out of that directory and list new source dirs here.
#
# checkPhase runs `cargo test`, so `nixos-rebuild build` proves parkerd as
# well as the config — the repo's test gate (root AGENTS.md, Building).
{ lib, rustPlatform }:

rustPlatform.buildRustPackage {
  pname = "parker";
  version = "0.1.0";

  src = lib.fileset.toSource {
    root = ./parkerd;
    fileset = lib.fileset.unions [
      ./parkerd/Cargo.toml
      ./parkerd/Cargo.lock
      ./parkerd/src
    ];
  };

  cargoLock.lockFile = ./parkerd/Cargo.lock;

  # Nothing to link — the dependency set (clap, serde, zbus) is pure Rust, so
  # the binary needs only libc. Runtime helpers it shells out to (hyprctl,
  # nvidia-smi, systemctl, df, date) are deliberately taken from PATH rather
  # than pinned: this is a user-session tool and its PATH is the session's.
  # That assumption gets revisited when it becomes a systemd service, where
  # PATH is much narrower.

  meta = {
    description = "parker — the machine's own senses";
    mainProgram = "parker";
    platforms = [ "x86_64-linux" ];
  };
}
