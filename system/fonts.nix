# Fonts. Before this file existed the system had zero icon-capable fonts —
# every nerd/material glyph (Noctalia widgets, workspace names, p10k)
# rendered as tofu boxes.
{ pkgs, ... }:
{
  fonts.packages = with pkgs; [
    nerd-fonts.meslo-lg # MesloLGS Nerd Font — terminal + p10k's native font
    nerd-fonts.symbols-only # fontconfig fallback for nerd glyphs anywhere
    material-symbols # Noctalia's icon style
    inter # UI sans
    jetbrains-mono # visage's face type (docs/visage/redesign); also the wallpaper stencil's
    noto-fonts
    noto-fonts-color-emoji
  ];
}
