# Ported from dotfiles/git/config. The holycode work identity (includeIf)
# stays on giskard — parker has no work checkouts.
{ pkgs, ... }:
{
  programs.git = {
    enable = true;

    ignores = [ "**/.claude/settings.local.json" ];

    settings = {
      # `programs.git.aliases` was renamed into settings (home-manager 2026).
      alias = {
        st = "status";
        co = "checkout";
        br = "branch";
        lg = "log --oneline --graph --decorate --all";
      };
      user.name = "Cabbagito";
      user.email = "you@example.com";
      init.defaultBranch = "main";
      diff.algorithm = "patience";
      pull.rebase = true;
      rebase.autoStash = true;
      push.default = "current";
      push.autoSetupRemote = true;
      merge.stat = true;
      merge.conflictstyle = "diff3";
      color.ui = "auto";
    };
  };

  programs.gh = {
    enable = true;
    extensions = [ pkgs.gh-dash ];
  };
}
