# Slippi Launcher (Melee netplay). Calibrated controller profiles + replays
# restored from the pre-install backup. isoPath: set when the Melee ISO
# resurfaces — the launcher works but can't launch a game without it.
{ inputs, ... }:
{
  imports = [ inputs.slippi.homeManagerModules.default ];
}
