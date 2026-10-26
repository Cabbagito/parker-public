# Ported from dotfiles/zsh/.zshrc (giskard). Dropped in transit: brew, nvm,
# bun, opencode, antigravity, thefuck, the `term` CLI (retired on parker,
# superseded by the `theme` command, home/theming), `ghost` (mac-only path).
{
  pkgs,
  lib,
  ...
}:
{
  programs.zsh = {
    enable = true;
    autosuggestion.enable = true;
    syntaxHighlighting.enable = true;

    oh-my-zsh = {
      enable = true;
      plugins = [ "git" ];
    };

    plugins = [
      {
        name = "powerlevel10k";
        src = pkgs.zsh-powerlevel10k;
        file = "share/zsh-powerlevel10k/powerlevel10k.zsh-theme";
      }
    ];

    shellAliases = {
      c = "clear";
      ll = "ls -l";
      vim = "nvim";
      lg = "lazygit";
      pwdc = "pwd | wl-copy";
      cy = "claude";
      ca = "claude agents";
      me = ''echo "$(whoami)@$(hostname -s)"'';
      this = ''echo "$(whoami)@$(hostname -s)"'';
      ff = "fastfetch";
      zk ="zellij kill-all-sessions -y; zellij delete-all-sessions -y";
      zj = "zellij kill-all-sessions -y >/dev/null 2>&1; zellij delete-all-sessions -y >/dev/null 2>&1; clear; _h=$(hostname -s); zellij -s $_h";
    };

    initContent = lib.mkMerge [
      # Instant prompt must run before everything else in .zshrc.
      (lib.mkOrder 500 ''
        if [[ -r "''${XDG_CACHE_HOME:-$HOME/.cache}/p10k-instant-prompt-''${(%):-%n}.zsh" ]]; then
          source "''${XDG_CACHE_HOME:-$HOME/.cache}/p10k-instant-prompt-''${(%):-%n}.zsh"
        fi
      '')
      ''
        # fzf default binds we don't use
        bindkey -r '^T'
        bindkey -r '^[c'

        # yazi with cwd-on-exit
        function y() {
          local tmp="$(mktemp -t "yazi-cwd.XXXXXX")"
          yazi "$@" --cwd-file="$tmp"
          if cwd="$(cat -- "$tmp")" && [ -n "$cwd" ] && [ "$cwd" != "$PWD" ]; then
            builtin cd -- "$cwd"
          fi
          rm -f -- "$tmp"
        }

        # The rebuild loop, readable: nom draws the build, nvd says what
        # changed. nrs = switch; nrb = build only, diffed against what is
        # running, into a throwaway link so no ./result is left behind.
        # Plain `sudo nixos-rebuild`, so the passwordless rule still applies.
        function nrs() {
          setopt local_options pipefail
          local before; before=$(readlink -f /run/current-system)
          sudo nixos-rebuild switch --flake /home/peki/parker#parker \
            --log-format internal-json -v "$@" |& nom --json || return
          nvd diff "$before" /run/current-system
        }
        function nrb() {
          setopt local_options pipefail
          local dir; dir=$(mktemp -d)
          nix build /home/peki/parker#nixosConfigurations.parker.config.system.build.toplevel \
            -o "$dir/result" --log-format internal-json -v "$@" |& nom --json \
            && nvd diff /run/current-system "$dir/result"
          local rc=$?; rm -rf "$dir"; return $rc
        }

        function ccc() {
          if zellij list-sessions 2>/dev/null | grep -q "ccc"; then
            zellij attach ccc
          else
            zellij -n ccc -s ccc
          fi
        }

        autoload -Uz add-zsh-hook
        auto_activate_venv() {
          if [[ -d .venv && -z "$VIRTUAL_ENV" ]]; then
            source .venv/bin/activate
          fi
        }
        add-zsh-hook chpwd auto_activate_venv

        [[ -f ~/.config/zsh/p10k.zsh ]] && source ~/.config/zsh/p10k.zsh
      ''
    ];
  };

  xdg.configFile."zsh/p10k.zsh".source = ./p10k.zsh;

  # fzf stays for its other uses, but Ctrl-R is atuin's now — an empty
  # history widget unbinds fzf's (home-manager warns if both claim it).
  # Ctrl-T and Alt-C stay unbound above: Ctrl-T is zellij's tab mode.
  programs.fzf = {
    enable = true;
    historyWidget.command = "";
  };
  programs.zoxide.enable = true;

  # History you can search: per directory, with exit codes and durations.
  # Ctrl-R only — Up stays plain zsh history, so muscle memory and
  # autosuggestions are untouched. Local only: no sync account, no update
  # pings. One-time after first switch: `atuin import zsh`.
  programs.atuin = {
    enable = true;
    flags = [ "--disable-up-arrow" ];
    settings = {
      auto_sync = false;
      update_check = false;
      style = "compact";
      inline_height = 20;
    };
  };

  # A cat with wings. Stylix themes it (do not set config.theme here); `cat`
  # itself stays cat.
  programs.bat.enable = true;
}
