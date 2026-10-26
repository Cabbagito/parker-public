# PC games outside Steam — Windows installers and portable exes run through
# GE-Proton (the one system/gaming.nix already hands to Steam). Heroic is
# the GUI (add exe, pick Proton, per-game env/prefix), umu-run the CLI
# equivalent underneath it. protontricks/winetricks run installers and
# verbs (vcrun, dotnet, d3dx9) inside an existing prefix; limo is a native
# mod manager for the games that need load orders. Layout (DECISIONS.md
# "PC games"): ~/gaming/{pc,prefixes,emulation} on the NVMe,
# ~/gaming/archive -> /mnt/hdd1 for installers and disc images. Heroic's
# config (~/.config/heroic) is runtime-owned, like the emulators'.
{ pkgs, ... }:
{
  home.packages = with pkgs; [
    heroic
    umu-launcher
    protontricks
    winetricks
    limo
    mangohud
    _7zz # ISOs, .7z/.rar mod archives — plain-files releases are `7zz x`
  ];
}
