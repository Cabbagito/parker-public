# The one user. Auto-login lives in system/desktop.nix (greetd) — unencrypted
# disk makes a login screen theater (DECISIONS.md). The password is for sudo,
# not for gates, and it is not in the repo: users stay mutable (the default),
# so it lives in /etc/shadow, set with `passwd` on install day (RUNBOOK.md).
{ pkgs, ... }:
{
  programs.zsh.enable = true;

  users.users.peki = {
    isNormalUser = true;
    shell = pkgs.zsh;
    extraGroups = [
      "wheel"
      "networkmanager"
      "docker"
    ];
    openssh.authorizedKeys.keys = [
      # giskard (MacBook) — LAN SSH from day one
      "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIKUE+kTHJ1GK8K/rb3b87/UBlb7Tg8AMOj7km4KzZW8s giskard"
    ];
  };

  # Passwordless nixos-rebuild only — the Claude-driven iteration loop
  # (rebuilds run non-interactively). Effectively root-equivalent for anyone
  # who can already edit the flake as peki, which is within the accepted
  # threat model (autologin; anyone at the keyboard is in). Everything else
  # sudo still asks.
  security.sudo.extraRules = [
    {
      users = [ "peki" ];
      commands = [
        {
          command = "/run/current-system/sw/bin/nixos-rebuild";
          options = [ "NOPASSWD" ];
        }
      ];
    }
  ];
}
