# Desktop role: Hyprland config, terminal, launcher, screenshot tools.
# Session plumbing (greetd, portals, UWSM) is system/desktop.nix's job.
{ pkgs, ... }:
{
  imports = [
    ./hyprland.nix
    ./ghostty.nix
    ./noctalia.nix
    ./xdg.nix
  ];

  # Cursor package/name/size come from stylix.cursor (system/theming.nix).
  # hyprcursor stays off: apple-cursor ships XCursor only, and advertising a
  # hyprcursor theme that isn't there just makes Hyprland warn and fall back.
  home.pointerCursor.hyprcursor.enable = false;

  # Screenshots: grim + slurp + wl-clipboard, satty for rare annotation
  # (DECISIONS.md). Bound in hyprland.nix.
  home.packages = with pkgs; [
    grim
    slurp
    satty
    wl-clipboard

    # `summon <space> <class> <command…>` — the Utility principle's "every
    # domain gets a summonable keyboard tool" (DECISIONS.md), in the shape
    # parker-face proved: the TUI lives in a ghostty on its own special
    # workspace, spawned on first use and only revealed after, so a second
    # press puts it away and the tool keeps its state in between. Placement
    # (floating, centred) is the window rule in hyprland.nix keyed on the
    # class, which must contain a dot or ghostty ignores it (see
    # parker-face). Binds: Super+A/S/D.
    (writeShellScriptBin "summon" ''
      space=$1 class=$2; shift 2
      have() {
        ${hyprland}/bin/hyprctl clients -j \
          | ${jq}/bin/jq -e --arg c "$class" 'any(.class == $c)' >/dev/null 2>&1
      }
      if ! have; then
        ${hyprland}/bin/hyprctl dispatch "hl.dsp.exec_cmd(\"ghostty --class=$class -e $*\")" >/dev/null
        # Wait for it to map, or the reveal shows an empty workspace and the
        # window then arrives `silent` and stays hidden.
        for _ in $(seq 30); do have && break; sleep 0.1; done
      fi
      exec ${hyprland}/bin/hyprctl dispatch "hl.dsp.workspace.toggle_special(\"$space\")" >/dev/null
    '')
  ];
}
