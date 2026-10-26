# Config, layouts, and plugin wasm blobs carried verbatim from dotfiles —
# the whole directory is linked as-is so relative plugin paths keep working.
# programs.zellij (not home.packages) so the stylix target can drop its
# theme file; the scheme inside is named `default`, which zellij auto-picks
# — config.kdl needs no theme line. HM writes no config.kdl while settings
# stay unset, so the hand link below keeps working.
{ ... }:
{
  programs.zellij.enable = true;

  xdg.configFile."zellij/config.kdl".source = ./zellij/config.kdl;
  xdg.configFile."zellij/layouts".source = ./zellij/layouts;
  xdg.configFile."zellij/plugins".source = ./zellij/plugins;
}
