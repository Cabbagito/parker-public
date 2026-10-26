# parker the entity — v0.
#
# A role beside terminal/, desktop/, tuis/: parker is the host, not a citizen,
# so it does not live in apps/ (PARKER.md, Repo placement).
#
# v0 is one binary with one subcommand, `parker sense`: read every input the
# machine exposes and print it. No daemon, no socket, no verbs. The point is
# to prove each input returns what we want and to settle the shape parker
# reads — the same functions become the daemon's, with a loop and a cache
# around them.
#
# The split PARKER.md anticipates began when there was a second thing:
# engine.nix (the opencode service, phase 3) lives beside this file, which
# keeps the binary, the face keybind, and the dev toolchain.
{
  pkgs,
  lib,
  osConfig,
  ...
}:
let
  themes = import ../../themes;
  preset = themes.forPolarity osConfig.parker.theme.polarity;

  # Most of the face is `Ink::Plain` and inherits the terminal's foreground,
  # so stylix themes it for free. Only the deliberate accents name a colour,
  # and those come from the active preset — the same move music.nix makes for
  # cava's gradient. `--set-default` so an explicit env still wins, and
  # `optionalString` because a preset need not carry an accent (macchiato).
  #
  # Wrapped rather than exported as a session variable: the face is spawned by
  # `hyprctl dispatch exec`, which uses Hyprland's environment and not the
  # calling shell's, so a session variable would silently not arrive.
  parker = pkgs.symlinkJoin {
    name = "parker-themed";
    paths = [ (pkgs.callPackage ./package.nix { }) ];
    nativeBuildInputs = [ pkgs.makeWrapper ];
    postBuild = ''
      wrapProgram $out/bin/parker \
        ${lib.optionalString (preset ? accent) ''--set-default PARKER_ACCENT "${preset.accent}" \''}
        --set-default PARKER_GOOD "8FB088" \
        --set-default PARKER_BAD "E04B4B"
    '';
    # symlinkJoin builds a *new* derivation and does not inherit the wrapped
    # package's meta, so `lib.getExe` falls back to this derivation's name and
    # resolves to `bin/parker-themed` — a path that has never existed. Every
    # getExe below then produced a script that exits 127 the moment it runs,
    # which is what a dead SUPER+P looks like: the bind fires, the script
    # cannot find what it was told to exec, and nothing at all happens.
    # package.nix sets the same mainProgram; this is only saying so again on
    # the far side of the wrapper.
    meta.mainProgram = "parker";
  };

  # visage, the same summon-or-reveal shape parker-face has, on its own
  # special workspace.
  #
  # The logic moved into the binary (`src/reach/visage.rs`) once there was a
  # second caller: SUPER+P is one way to ask for the face, and `parker visage`
  # typed over SSH is another. They are the same gesture, so they are one
  # command with no flags, and this script is only the name Hyprland binds to.
  # Everything it used to do in jq and sleep loops (find the window, wait out
  # a cold Tauri start, bail rather than reveal an empty workspace) lives
  # there instead. hyprctl is put on PATH because parker takes it from PATH
  # deliberately (package.nix) and this runs from Hyprland's environment, not
  # a shell's.
  parker-visage = pkgs.writeShellScriptBin "parker-visage" ''
    export PATH="${pkgs.hyprland}/bin:$PATH"
    exec ${lib.getExe parker} visage
  '';
in
{
  imports = [
    ./engine.nix
    ./badge.nix
    ./code.nix
    ./porthole.nix
  ];

  home.packages = [
    parker

    # SUPER+P: show parker's face, on a special workspace nothing else reaches.
    #
    # The face is started at login (exec-once in home/desktop/hyprland.nix) and
    # simply lives there, so this is normally just a reveal. Parker's face is
    # not something that gets *created* when you look at it — it is running
    # whenever the machine is, which is the whole premise, and it also means
    # the face you summon is mid-animation rather than starting cold.
    #
    # The spawn branch is the recovery path, for when it has been closed or
    # died. Placement, fullscreen and the font come from the window rule in
    # home/desktop/hyprland.nix, keyed on the class below.
    #
    # `--class` MUST contain a dot: ghostty silently ignores it otherwise (it
    # wants a valid GTK application id), which is exactly how the first version
    # of this broke — the window kept the ghostty class, no rule matched, and
    # it opened on whatever workspace you were already on.
    (pkgs.writeShellScriptBin "parker-face" ''
      class=com.parker.face
      if ! ${pkgs.hyprland}/bin/hyprctl clients -j \
        | ${pkgs.jq}/bin/jq -e --arg c "$class" 'any(.class == $c)' >/dev/null 2>&1; then
        ${pkgs.hyprland}/bin/hyprctl dispatch \
          "hl.dsp.exec_cmd(\"ghostty --class=$class --font-size=32 --window-padding-x=0 --window-padding-y=0 -e parker face --animate\")"
        sleep 0.4
      fi
      exec ${pkgs.hyprland}/bin/hyprctl dispatch 'hl.dsp.workspace.toggle_special("parker")'
    '')

    # visage, same summon-or-reveal shape on its own special workspace.
    # Scaffolding: the binary is the dev-loop build under the repo checkout
    # (cargo tauri build --no-bundle), not a nix artifact — the visage
    # package derivation replaces this path when it lands. The class is the
    # live app_id `visage`, not the tauri identifier (visage/AGENTS.md).
    parker-visage

    # The first thing the machine does after login (exec-once in
    # home/desktop/hyprland.nix, last in the list): land on space 3, then let
    # visage take the screen.
    #
    # The order is the point. A special workspace is an overlay, so whatever
    # workspace is underneath when visage appears is the one you fall back to
    # when it goes away — switch to 3 first and dismissing the face drops you
    # on the terminal instead of on space 1, which is where a fresh session
    # would otherwise leave you (DECISIONS.md, Session).
    (pkgs.writeShellScriptBin "parker-greet" ''
      export PATH="${pkgs.hyprland}/bin:$PATH"
      ${pkgs.hyprland}/bin/hyprctl dispatch 'hl.dsp.focus({ workspace = 3 })'
      exec ${lib.getExe parker} visage
    '')


    # `reach show`'s projectors: images float in imv, PDFs open in zathura,
    # text panes render with glow (already on PATH via zen.nix). Installed
    # with the verb rather than waiting on the media-stack step — a verb
    # whose viewers are missing is a verb that lies.
    pkgs.imv
    pkgs.zathura

    # The inner dev loop is `cd ~/parker/home/parker/parkerd && cargo run` —
    # ~0.2s incremental, versus ~20s to go through nixos-rebuild. The nix
    # build is for deploying, not for iterating. nixpkgs' toolchain is
    # current (rustc 1.97), so no rust-overlay input is needed.
    pkgs.rustc
    pkgs.cargo
    pkgs.rust-analyzer
    pkgs.clippy
    pkgs.rustfmt
  ];

  # Without this rust-analyzer cannot resolve std.
  home.sessionVariables.RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";

  # `parker brain`'s catalog (parkerd's brain.rs): brains.nix compiled to
  # JSON, read-only like every other config here. Which brain is in use is
  # not in it — that is state, so switching needs no rebuild.
  xdg.configFile."parker/brains.json".text = builtins.toJSON (import ./brains.nix);

  # Claude Code no longer gets `parker mcp`. It was deliberately the first
  # client — the seam proven by an engine parker does not ship with — but
  # that experiment is over and the verbs belong to parker instances only
  # (`parker code` and the engine). Claude Code is the generic coding tool;
  # if it needs to know the machine, that is a conversation, not a socket.
  # The old activation entry patched ~/.claude.json; its removal is a one-off
  # `jq 'del(.mcpServers.parker)'` since home-manager cannot un-write
  # app-owned state.
}
