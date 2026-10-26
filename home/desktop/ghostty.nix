# ghostty — the terminal (space 3 citizen with zellij). Minimal on purpose:
# theme, font, and size come from stylix (fonts.monospace, sizes.terminal),
# background-opacity from stylix.opacity.terminal (system/theming.nix).
{ ... }:
{
  programs.ghostty = {
    enable = true;
    enableZshIntegration = true;
    settings = {
      confirm-close-surface = false;
      # No toasts: config-reload flashes on every theme set (SIGUSR2), and
      # clipboard-copy fires on every copy — both read as noise.
      app-notifications = "no-clipboard-copy,no-config-reload";
    };
  };
}
