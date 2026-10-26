# Boot, nix itself, locale, and the small always-on system services.
{ pkgs, ... }:
{
  boot.loader.systemd-boot.enable = true;
  boot.loader.systemd-boot.configurationLimit = 20;
  boot.loader.efi.canTouchEfiVariables = true;

  nix.settings.experimental-features = [
    "nix-command"
    "flakes"
  ];
  # Noctalia's binary cache — its C++/GLES build is heavy. It serves stock
  # builds only: while home/desktop/noctalia.nix carries local patches, the
  # patched shell builds here anyway (the accepted cost recorded there).
  nix.settings.extra-substituters = [ "https://noctalia.cachix.org" ];
  nix.settings.extra-trusted-public-keys = [
    "noctalia.cachix.org-1:pCOR47nnMEo5thcxNDtzWpOxNFQsBRglJzxWPp3dkU4="
  ];
  nixpkgs.config.allowUnfree = true;

  # Old generations, pruned weekly. configurationLimit above only trims the
  # boot menu; it never deleted a generation, which is how 95 of them came to
  # hold / at 83% (Oct 2026).
  #
  # nh's clean *service* only — not nh itself. nh would replace
  # nixos-rebuild, and it elevates as `sudo env … <store path>`, which the
  # passwordless rule in users.nix cannot match without becoming root for
  # anything; parker, `theme set` and nrs all rely on that rule. Count-based
  # on purpose: `--keep 10` still holds the generation from before a monthly
  # update however long the gap, where nix.gc's --delete-older-than would
  # not. (nh's module warns if nix.gc.automatic is on as well.)
  programs.nh.clean = {
    enable = true;
    dates = "weekly";
    extraArgs = "--keep 10 --keep-since 14d";
  };
  # Hardlink identical store files on a weekly timer, rather than
  # auto-optimise-store, which pays the same cost inside every build.
  nix.optimise.automatic = true;

  time.timeZone = "Europe/Belgrade";
  i18n.defaultLocale = "en_US.UTF-8";

  zramSwap.enable = true;

  # systemd-oomd runs by default and watches nothing until slices opt in
  # (oomctl listed zero cgroups). Root and user slices are Fedora's defaults:
  # a runaway — a Wine prefix, a leaky build — is killed at sustained 80%
  # memory pressure instead of freezing the desktop first.
  systemd.oomd.enableRootSlice = true;
  systemd.oomd.enableUserSlices = true;

  # Terminfo for every terminal nixpkgs knows about, so SSH sessions from
  # non-default terminals (ghostty on the MacBook) get a working TERM.
  environment.enableAllTerminfo = true;

  # Prebuilt dynamic binaries (uv-installed wheels, Mason LSP servers) link
  # against this instead of failing on the missing FHS loader.
  programs.nix-ld.enable = true;

  # Root-level basics; everything user-facing lives in home/.
  environment.systemPackages = with pkgs; [
    git
    vim
    curl
    wget
  ];
}
