# Claude Desktop (official Linux beta repackaged by aaddrick) — space 2
# citizen. FHS variant so MCP server subprocesses (node/uvx) resolve.
# `claude-desktop --doctor` exists for diagnostics.
{ pkgs, inputs, ... }:
{
  home.packages = [
    inputs.claude-desktop.packages.${pkgs.stdenv.hostPlatform.system}.claude-desktop-fhs
  ];
}
