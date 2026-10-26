# Spotify — space 4 citizen. Sets its window class late, so workspace
# placement happens via the exec-once dispatcher, never a class rule.
{ pkgs, ... }:
{
  home.packages = with pkgs; [ spotify ];
}
