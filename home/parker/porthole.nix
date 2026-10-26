# visage on the tailnet — parker's face for giskard and the phone
# (docs/visage/porthole/README.md).
#
# Three things: the page (the same dist the Tauri app embeds, plus the
# JetBrains Mono faces a screen without the font needs), the server
# (porthole, the second frame on visage/door), and the user service that
# runs it on loopback. The way in from the tailnet is `tailscale serve`,
# which is system-level and lives in system/tailscale.nix. The port is the
# one number the two files share.
#
# Nothing here can be reached from the LAN or the tailnet directly:
# porthole refuses to bind anything but loopback, because the identity
# header its gate checks is only trustworthy when serve is the sole way in.
{
  pkgs,
  lib,
  config,
  ...
}:
let
  port = 4097;

  # Who may in, and from where. The tailnet's one login (`tailscale status`)
  # and the page's own origin; a write carrying any other Origin is refused.
  login = "you@example.com";
  origin = "https://parker.tail1b2a09.ts.net";

  parker = pkgs.callPackage ./package.nix { };

  # The fileset is the same discipline as package.nix: only build inputs, so
  # touching AGENTS.md or the Tauri shell does not rebuild the page.
  page = pkgs.buildNpmPackage {
    pname = "visage-page";
    version = "0.1.0";
    src = lib.fileset.toSource {
      root = ./visage;
      fileset = lib.fileset.unions [
        ./visage/package.json
        ./visage/package-lock.json
        ./visage/index.html
        ./visage/vite.config.js
        ./visage/src
        ./visage/public
      ];
    };
    # Recompute after any package-lock.json change:
    #   nix run --inputs-from ~/parker nixpkgs#prefetch-npm-deps -- home/parker/visage/package-lock.json
    npmDepsHash = "sha256-NPWtcGHd7c+ETZBj/nBrHo+YHlm8svrYu7r/jzHkxCw=";

    # The page is files, not a node package: dist is the output. The fonts
    # are the ones app.css names as `JetBrains Mono Web`.
    installPhase = ''
      runHook preInstall
      cp -r dist $out
      mkdir -p $out/fonts
      for face in Regular Italic Bold BoldItalic; do
        cp ${pkgs.jetbrains-mono}/share/fonts/opentype/JetBrainsMono-$face.otf $out/fonts/
      done
      runHook postInstall
    '';
  };

  # Two crates from one tree: web/ is built, door/ comes along as its path
  # dependency. checkPhase runs web's `cargo test`, so the gate's tests —
  # including the one that asks the assembled router — gate the rebuild.
  server = pkgs.rustPlatform.buildRustPackage {
    pname = "porthole";
    version = "0.1.0";
    src = lib.fileset.toSource {
      root = ./visage;
      fileset = lib.fileset.unions [
        ./visage/door/Cargo.toml
        ./visage/door/src
        ./visage/porthole/Cargo.toml
        ./visage/porthole/Cargo.lock
        ./visage/porthole/src
      ];
    };
    cargoLock.lockFile = ./visage/porthole/Cargo.lock;
    cargoRoot = "porthole";
    buildAndTestSubdir = "porthole";
    meta.mainProgram = "porthole";
  };
in
{
  systemd.user.services.parker-porthole = {
    Unit = {
      Description = "visage on the tailnet — parker's face for giskard and the phone";
      After = [ "parker-engine.service" ];
    };

    Service = {
      ExecStart = lib.concatStringsSep " " [
        (lib.getExe server)
        "--listen 127.0.0.1:${toString port}"
        "--dist ${page}"
        "--allow ${login}"
        "--origin ${origin}"
      ];
      # `sense` forks nvidia-smi, systemctl and hyprctl from PATH
      # (package.nix), so the service gets the session's, as the engine does.
      Environment = [
        "PARKER_BIN=${lib.getExe parker}"
        "PATH=${config.home.profileDirectory}/bin:/run/current-system/sw/bin"
      ];
      Restart = "on-failure";
      RestartSec = 2;
    };

    Install.WantedBy = [ "default.target" ];
  };
}
