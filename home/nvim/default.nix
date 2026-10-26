# Config tree carried as-is from dotfiles; lazy.nvim keeps managing plugins
# at runtime (accepted undeclared pocket — DECISIONS.md). The lockfile is
# redirected to a writable path in config/lua/config/lazy.lua because
# ~/.config/nvim is a read-only store symlink here.
{ pkgs, ... }:
{
  home.packages = with pkgs; [
    neovim
    # runtime deps: treesitter compiles grammars, telescope searches,
    # Mason fetches LSP servers (nix-ld covers their dynamic linking)
    gcc
    gnumake
    unzip
    ripgrep
    fd
    nodejs
    stylua
  ];

  xdg.configFile."nvim".source = ./config;

  home.sessionVariables.EDITOR = "nvim";
}
