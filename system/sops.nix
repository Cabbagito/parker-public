# sops-nix is wired at the flake level; v0 deliberately ships ZERO secrets
# (DECISIONS.md — nothing at install depends on a secret; wifi is one
# interactive nmtui login). Still zero as of July 2026 — never bootstrapped.
#
# The repo is PRIVATE (DECISIONS.md, corrected July 2026), so this is no
# longer the only way to hold a credential: rotatable, scoped, spend-capped
# keys may sit in the repo in plaintext. Reach for sops when a secret is
# worth encrypting at rest, not reflexively for every token.
#
# Bootstrap, post-first-boot (Claude-on-parker's job):
#   1. age key from the host SSH key:
#        nix run nixpkgs#ssh-to-age -- -i /etc/ssh/ssh_host_ed25519_key.pub
#   2. .sops.yaml recipients = personal age key + parker's host key
#   3. First secret:
#        sops.defaultSopsFile = ../secrets/secrets.yaml;
#        sops.age.sshKeyPaths = [ "/etc/ssh/ssh_host_ed25519_key" ];
#
# Rule: only rotatable, low-blast-radius secrets belong in this repo at all.
{ ... }:
{
}
