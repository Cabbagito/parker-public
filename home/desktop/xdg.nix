# Declared home directory structure. The zellij ai tab cd's into Documents;
# Desktop/Templates/Public are pointed at $HOME so they never get created —
# a tiling desktop has no desktop.
{ config, ... }:
{
  xdg.userDirs = {
    enable = true;
    createDirectories = true;
    desktop = config.home.homeDirectory;
    templates = config.home.homeDirectory;
    publicShare = config.home.homeDirectory;
  };
}
