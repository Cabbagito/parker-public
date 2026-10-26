# qBittorrent — summonable, not furniture. Qt app, so it inherits the Stylix
# qt platform theme along with everything else.
{ pkgs, ... }:
{
  home.packages = with pkgs; [ qbittorrent ];

  # First (and so far only) xdg.mimeApps consumer — enabling it makes
  # ~/.config/mimeapps.list managed like every other config here.
  xdg.mimeApps = {
    enable = true;
    defaultApplications = {
      "application/x-bittorrent" = "org.qbittorrent.qBittorrent.desktop";
      "x-scheme-handler/magnet" = "org.qbittorrent.qBittorrent.desktop";
    };
  };
}
