# Home layer entry point. Composes roles, never configures.
{ ... }:
{
  imports = [
    ./terminal
    ./nvim
    ./tuis
    ./apps
    ./desktop
    ./theming
    ./parker
  ];

  home.username = "peki";
  home.homeDirectory = "/home/peki";

  # First-install release. Never change this on updates.
  home.stateVersion = "26.05";

  programs.home-manager.enable = true;
}
