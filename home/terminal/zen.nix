# `zen` — a Claude-generated interstitial for the zellij zen tab: random
# kind × topic, rendered with glow. Script is the canonical copy in zen.sh.
{ pkgs, ... }:
{
  home.packages = [
    pkgs.glow
    (pkgs.writeShellScriptBin "zen" (builtins.readFile ./zen.sh))
  ];
}
