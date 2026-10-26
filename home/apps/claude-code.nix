# Claude Code — the maintenance model for this whole machine.
# settings.json is a read-only store symlink; runtime state lives in
# ~/.claude.json which the app owns. If claude-code ever complains about
# writing settings.json, switch to an activation-script copy.
{
  pkgs,
  lib,
  config,
  ...
}:
let
  # parker's verbs (face, sense, look…) for Claude Code, through the profile
  # path rather than a store path: a store path pins whatever build was
  # current when it was registered — that is how this ran a six-verb parker
  # for two months — and the weekly gc would eventually delete it outright.
  parkerMcp = builtins.toJSON {
    type = "stdio";
    command = "${config.home.profileDirectory}/bin/parker";
    args = [ "mcp" ];
  };
in
{
  home.packages = with pkgs; [ claude-code ];

  home.file.".claude/settings.json".source = ./claude/settings.json;
  home.file.".claude/statusline-command.sh" = {
    source = ./claude/statusline-command.sh;
    executable = true;
  };

  # Claude Code never saves the trust-dialog answer for $HOME itself
  # (session-only by design) but it does honor the flag when present, so
  # re-pin it on every switch in case ~/.claude.json ever gets regenerated.
  home.activation.claudeTrustHome = lib.hm.dag.entryAfter [ "writeBoundary" ] ''
    claudeState="$HOME/.claude.json"
    if [ -f "$claudeState" ] && ! ${pkgs.jq}/bin/jq -e \
        '.projects[env.HOME].hasTrustDialogAccepted == true' "$claudeState" >/dev/null 2>&1; then
      run sh -c '${pkgs.jq}/bin/jq ".projects[env.HOME].hasTrustDialogAccepted = true" "$1" > "$1.hm-new" && mv "$1.hm-new" "$1"' sh "$claudeState"
    fi
  '';

  # Same file, same shape: the parker MCP registration lives in the
  # app-owned ~/.claude.json, so it is asserted on every switch rather than
  # declared. Only rewritten when it differs, so Claude Code's own writes
  # to the file are left alone.
  home.activation.claudeParkerMcp = lib.hm.dag.entryAfter [ "claudeTrustHome" ] ''
    claudeState="$HOME/.claude.json"
    want='${parkerMcp}'
    if [ -f "$claudeState" ] && ! ${pkgs.jq}/bin/jq -e --argjson want "$want" \
        '.mcpServers.parker == $want' "$claudeState" >/dev/null 2>&1; then
      run sh -c '${pkgs.jq}/bin/jq --argjson want "$2" ".mcpServers.parker = \$want" "$1" > "$1.hm-new" && mv "$1.hm-new" "$1"' sh "$claudeState" "$want"
    fi
  '';
}
