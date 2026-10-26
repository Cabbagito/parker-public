# PipeWire owns audio (DECISIONS.md). No JACK until a use case shows up.
{ ... }:
{
  services.pulseaudio.enable = false;
  # Realtime scheduling for the audio graph.
  security.rtkit.enable = true;

  services.pipewire = {
    enable = true;
    alsa.enable = true;
    alsa.support32Bit = true;
    pulse.enable = true;

    # Wine's DirectSound emulation garbles/crackles when the pulse shim
    # feeds it tiny dynamic buffers (GTA IV was the symptom). Floor the
    # quantum at 1024 samples (~21ms) for pulse clients; native apps are
    # unaffected below the floor only if they ask for less — acceptable
    # trade on a desktop with no pro-audio use case.
    extraConfig.pipewire-pulse."92-wine-dsound-fix" = {
      "pulse.properties" = {
        "pulse.min.req" = "1024/48000";
        "pulse.min.quantum" = "1024/48000";
      };
    };

    # NVIDIA HDMI audio resumes badly from idle suspension: the codec
    # re-negotiates and Wine DirectSound clients desync into scrambled
    # playback until buffers realign. Never suspend ALSA nodes; pairs
    # with power_save=0 below which keeps the codec itself powered.
    wireplumber.extraConfig."99-disable-suspend" = {
      "monitor.alsa.rules" = [
        {
          matches = [
            { "node.name" = "~alsa_output.*"; }
            { "node.name" = "~alsa_input.*"; }
          ];
          actions.update-props = {
            "session.suspend-timeout-seconds" = 0;
          };
        }
      ];
    };
  };

  boot.extraModprobeConfig = ''
    options snd_hda_intel power_save=0 power_save_controller=N
  '';
}
