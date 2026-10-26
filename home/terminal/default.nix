# Terminal-land: shell, git, multiplexer. Alt is the modifier here; Super
# belongs to the OS (DECISIONS.md).
{ pkgs, ... }:
{
  imports = [
    ./zsh.nix
    ./git.nix
    ./zellij.nix
    ./zen.nix
  ];

  # uv owns Python — versions, venvs, packages (DECISIONS.md, Domains).
  home.packages = with pkgs; [
    uv
    # nrs / nrb (zsh.nix): build progress and the "what changed" diff
    nix-output-monitor
    nvd
  ];
}
