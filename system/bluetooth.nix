# Bluetooth: the Intel CNVi radio (hci0) and its firmware were already live
# at the kernel level; this is just the BlueZ daemon on top. First customer
# is the DualSense (see system/gaming.nix for the controller-side rules).
# Control via bluetui (home/tuis), glance via Noctalia's control center.
{ ... }:
{
  hardware.bluetooth = {
    enable = true;
    powerOnBoot = true;
    settings.General = {
      # Battery level of paired devices via UPower (bluetui/Noctalia show it).
      Experimental = true;
      # Let a paired pad reconnect with a PS-button press even when the
      # radio has been idle a while.
      FastConnectable = true;
    };
  };
}
