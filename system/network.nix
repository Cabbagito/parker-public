# NetworkManager (wifi-first — no cable at parker's location) + sshd as a
# system service, independent of any graphical session.
{ ... }:
{
  networking.networkmanager.enable = true;

  services.openssh = {
    enable = true;
    settings = {
      PasswordAuthentication = false;
      KbdInteractiveAuthentication = false;
    };
  };
}
