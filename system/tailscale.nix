# Tailscale — un-deferral of the DECISIONS.md remote-node use case
# (July 2026). Login state lives in /var/lib/tailscale after one
# interactive `sudo tailscale up`, same pattern as the nmtui wifi login;
# it survives reboots and rebuilds. An authKeyFile via sops is the
# zero-touch-reinstall purity upgrade, not a prerequisite.
#
# Remote SSH is the existing sshd (network.nix) reached over the tailnet —
# trustedInterfaces below, MagicDNS name "parker". No new ports on the LAN.
#
# Landmine noted for later: if NordVPN or an exit node ever lands, set
# networking.firewall.checkReversePath = "loose" — rpfilter drops Tailscale
# traffic under other VPNs.
{ config, lib, ... }:
let
  tailscale = lib.getExe config.services.tailscale.package;
in
{
  services.tailscale = {
    enable = true;
    openFirewall = true; # UDP 41641 for direct connections (else DERP relay)
    # Peter may bring the tailnet up and down without root: visage's ^,
    # has a tailscale row (docs/visage/controls/README.md), and parkerd runs
    # `tailscale up/down` as him. The row is the tower's alone — from the
    # porthole it would cut the line the page rides on.
    extraSetFlags = [ "--operator=peki" ];
  };
  networking.firewall.trustedInterfaces = [ "tailscale0" ];

  # visage on the tailnet (docs/visage/porthole/README.md): https://parker.<tailnet>.ts.net
  # → porthole on 127.0.0.1:4097, the user service in
  # home/parker/porthole.nix. The port is the one number the two files
  # share. serve adds the `Tailscale-User-Login` header porthole's gate
  # checks, and strips any copy a client sent.
  #
  # Not nixpkgs' `services.tailscale.serve`: that option configures
  # Tailscale *Services* (`svc:` virtual IPs on tagged hosts), and traffic
  # from tagged devices carries no identity headers. And never Funnel — that
  # is the public internet, and Funnel traffic carries no identity either.
  #
  # Needs HTTPS certificates enabled for the tailnet (admin console → DNS),
  # once. Until then the CLI waits for someone to enable them, hence the
  # start timeout: a rebuild must not hang on a browser tab nobody opened.
  # The serve config itself persists in tailscaled's state, so stopping this
  # unit (deleting it from the config included) turns it off explicitly.
  systemd.services.tailscale-serve-porthole = {
    description = "Publish visage to the tailnet (tailscale serve)";
    after = [ "tailscaled.service" ];
    wants = [ "tailscaled.service" ];
    wantedBy = [ "multi-user.target" ];
    serviceConfig = {
      Type = "oneshot";
      RemainAfterExit = true;
      ExecStart = "${tailscale} serve --bg --yes --https=443 http://127.0.0.1:4097";
      ExecStop = "${tailscale} serve --https=443 off";
      TimeoutStartSec = 30;
      # tailscaled can be up before it can take serve config at boot.
      Restart = "on-failure";
      RestartSec = 15;
    };
  };
}
