# Declared, not furniture: socket-activated on first `docker` command,
# nothing running at boot.
{ ... }:
{
  virtualisation.docker = {
    enable = true;
    enableOnBoot = false;
  };
}
