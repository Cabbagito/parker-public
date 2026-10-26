# Pre-baked without machine access (standard desktop module set + facts from
# the old install). Install day: run `nixos-generate-config --no-filesystems`
# in the live env and diff against this file before installing.
{ ... }:
{
  nixpkgs.hostPlatform = "x86_64-linux";

  boot.initrd.availableKernelModules = [
    "nvme"
    "xhci_pci"
    "ahci"
    "usbhid"
    "usb_storage"
    "sd_mod"
  ];
  boot.kernelModules = [ "kvm-intel" ];

  hardware.cpu.intel.updateMicrocode = true;
  # iwlwifi firmware — wifi is the only uplink at parker's location.
  hardware.enableRedistributableFirmware = true;

  # Data drives: remounted, never touched by the install. nofail so a dead
  # drive can't block boot.
  fileSystems."/mnt/hdd1" = {
    device = "/dev/disk/by-id/ata-WDC_WD10EZEX-08WN4A0_WD-WCC6Y2EJ0D5V-part1";
    fsType = "ext4";
    options = [ "nofail" ];
  };
  fileSystems."/mnt/hdd2" = {
    device = "/dev/disk/by-id/ata-WDC_WD5000AAKX-001CA0_WD-WCAYUEK84641-part1";
    fsType = "ext4";
    options = [ "nofail" ];
  };
  fileSystems."/mnt/ssd" = {
    device = "/dev/disk/by-id/ata-KINGSTON_SHFS37A120G_50026B777804CE75-part1";
    fsType = "ext4";
    options = [ "nofail" ];
  };
}
