# parker code — opencode's own TUI as parker's coding face.
#
# The coding-shaped sibling of engine.nix: same mind (mind.nix), same verbs
# (parker mcp), but here parker works in modes — ghost, wired and the
# subagents below — each with its own short prompt, and the repo's
# AGENTS.md files come in the ordinary way, discovered from the cwd.
# `parker code` (parkerd) execs opencode pointing at this config.
#
# The theme is generated from the active Stylix scheme, so it IS the
# machine's theme — redline by night, hangar by day — and follows
# `theme set` through the same rebuild everything else does. Fork nothing:
# themes, MCP, agents, commands and the home-screen wordmark are all plain
# opencode config.
#
# opencode splits its config in two. Everything about the agent (providers,
# MCP, agents, commands, permissions, server plugins) is config.json, reached
# by OPENCODE_CONFIG; everything about the terminal UI (theme, keybinds, TUI
# plugins, notification sounds) is tui.json, reached by OPENCODE_TUI_CONFIG.
# parkerd sets both. `theme` in config.json still works but is deprecated —
# opencode migrates it out on first run by writing a tui.json next to the
# config file, which for us meant an untracked file in ~/.config/parker that
# home-manager did not own. Declaring it here instead is the same setting with
# none of the drift.
#
# Four plugins, and the split between them matters:
#
#   code-logo.tsx    TUI     the wordmark (home_logo)
#   code-status.tsx  TUI     the statusline and the MACHINE panel
#   code-theme.tsx   TUI     live day/night without a restart
#   code-hooks.ts    SERVER  the agent loop — injection, env, guards, the face
#
# TUI plugins go in tui.json's `plugin`; the server plugin goes in
# config.json's `plugin`. Same key name, two different files, and swapping
# them silently loads nothing. docs/OPENCODE.md is the capability reference
# for the whole seam.
{
  pkgs,
  lib,
  config,
  ...
}:
let
  parker = pkgs.callPackage ./package.nix { };
  mind = import ./mind.nix { inherit pkgs lib; };
  json = pkgs.formats.json { };

  # parker's brains (brains.nix), as opencode's model picker should see them:
  # `provider/model` split apart, and the model keeping the brain's name.
  # The cheap model rides along whether or not a brain names it — it does
  # the session titles and the probe subagent below.
  brains = import ./brains.nix;
  cheap = "anthropic/claude-haiku-5-5";
  split = model: {
    provider = lib.head (lib.splitString "/" model);
    id = lib.concatStringsSep "/" (lib.tail (lib.splitString "/" model));
  };
  picker = lib.mapAttrsToList (name: brain: split brain.model // { inherit name; }) brains.brains;
  pickable = picker ++ [ (split cheap) ];
  providers = lib.unique (map (m: m.provider) pickable);

  c = config.lib.stylix.colors;
  hex = base: "#${c.${base}}";

  # base16 → opencode's theme slots. The scheme's own comments drive the
  # mapping: base08 is "red (accent)", base0B "green, diffs only", etc.
  theme = json.generate "parker-opencode-theme.json" {
    "$schema" = "https://opencode.ai/theme.json";
    theme = {
      primary = hex "base08";
      secondary = hex "base0D";
      accent = hex "base09";
      error = hex "base08";
      warning = hex "base0A";
      success = hex "base0B";
      info = hex "base0D";
      text = hex "base05";
      textMuted = hex "base04";
      background = hex "base00";
      backgroundPanel = hex "base01";
      backgroundElement = hex "base02";
      border = hex "base02";
      borderActive = hex "base03";
      borderSubtle = hex "base01";
      diffAdded = hex "base0B";
      diffRemoved = hex "base08";
      diffContext = hex "base04";
      diffHunkHeader = hex "base03";
      markdownText = hex "base05";
      markdownHeading = hex "base08";
      markdownLink = hex "base0D";
      markdownLinkText = hex "base0C";
      markdownCode = hex "base09";
      markdownBlockQuote = hex "base04";
      markdownEmph = hex "base0E";
      markdownStrong = hex "base06";
      markdownHorizontalRule = hex "base03";
      markdownListItem = hex "base05";
      markdownListEnumeration = hex "base0C";
      markdownCodeBlock = hex "base05";
      syntaxComment = hex "base03";
      syntaxKeyword = hex "base08";
      syntaxFunction = hex "base0D";
      syntaxVariable = hex "base05";
      syntaxString = hex "base0B";
      syntaxNumber = hex "base09";
      syntaxType = hex "base0A";
      syntaxOperator = hex "base05";
      syntaxPunctuation = hex "base04";
    };
  };

  # ── the crew ──────────────────────────────────────────────────────────
  #
  # opencode ships two primary agents, `plan` and `build`, and they are
  # ordinary config: `disable` turns a built-in off and any other key defines
  # a new one, so renaming them is really replacing them.
  #
  # Two modes, because there are only two honest answers to "what may parker
  # touch right now":
  #
  #   ghost  read-only. moves through the system and leaves nothing.
  #   wired  jacked in. full tools. the default.
  #
  # There was briefly a third, `calibrate`. It was a mistake: fmt → clippy →
  # test → build is a *checklist*, not a way of operating, and a mode you
  # switch into to run a fixed sequence is a command wearing the wrong hat.
  # It lives on as `/calibrate` below, which is what it always was.
  #
  # The subagents are the ones a primary hands work to, by @mention or on its
  # own judgement. They are deliberately narrow — a subagent that can do
  # anything is just another wired with a worse prompt.
  #
  #   probe    find it. cheap model, read-only, no opinions.
  #   redteam  try to break it. read-only, adversarial by construction.
  #   autopsy  it is dead and nobody knows why. full tools.
  #
  # A note on the permissions below: `ghost` is a *posture*, not a sandbox.
  # It cannot edit, write or patch, which is the part that matters, but its
  # bash is wide open because reading the world is the whole job and an
  # allow-list of read commands is a list you are still editing a year later.
  # Anything that genuinely must not happen is enforced in code-hooks.ts,
  # where it applies to every agent and cannot be switched off by a Tab.
  #
  # `instructions` (parker's mind, mind.nix) appends to every one of these,
  # so each prompt below only says what makes this mode different from
  # parker generally. None of them re-states the persona or the build rules
  # (root AGENTS.md); that would be saying it twice.
  agents = {
    plan.disable = true;
    build.disable = true;

    ghost = {
      mode = "primary";
      description = "Read-only. Survey, trace, report — touches nothing.";
      color = "secondary";
      prompt = ''
        You are running as ghost. You are reading the machine, not changing it.

        Produce understanding: what exists, where it lives, what depends on
        what, and which of the several plausible approaches actually fits
        this repo. Read DECISIONS.md before proposing anything structural —
        it records why things are the way they are.

        You cannot write, edit or patch, and that is the point: a plan you
        cannot start implementing halfway through is a plan you finish.

        End with the approach you would take and the one you rejected, and
        say plainly what you are still unsure about. Do not pad it into a
        document — this house prefers a short answer that is load-bearing.
      '';
      permission = {
        edit = "deny";
        write = "deny";
        patch = "deny";
      };
    };

    wired = {
      mode = "primary";
      description = "Jacked in. Full tools — the default.";
      color = "primary";
      prompt = ''
        You are wired in. Make the change.

        The warden docs are the contract: AGENTS.md at the repo root, then the
        nearest AGENTS.md to whatever you are touching. They are not
        suggestions — most of them record a specific way this repo has already
        been broken once.

        `nixos-rebuild build --flake $PARKER_FLAKE` is the test gate and it is
        yours to run. `switch` is Peter's; propose it, never run it.

        Flakes only see git-tracked files: `git add` a new source before you
        build, or the build will not see it and the error will not say so.

        Write the comment that explains why, not the one that restates what.
      '';
      permission = {
        edit = "allow";
        bash = {
          "*" = "allow";
          # Only two gates, and neither is about competence: `sudo` leaves
          # userland and `git push` leaves the machine. Everything destructive
          # *within* the repo is recoverable from git, so it does not ask.
          "sudo *" = "ask";
          "git push*" = "ask";
        };
      };
    };

    probe = {
      mode = "subagent";
      description = "Find where something lives in this repo. Fast, cheap, read-only. Use when you need a location, not an analysis.";
      color = "info";
      # The title model. A search that needs the good model is not a search,
      # it is a question, and it should go to ghost instead.
      model = cheap;
      prompt = ''
        Locate things. Return file paths and line numbers and the two or three
        lines of context that prove you found the right one.

        Do not explain the code, do not judge it, do not suggest changes. If
        you cannot find it, say where you looked. A wrong confident answer
        costs more than "not found".
      '';
      permission = {
        edit = "deny";
        write = "deny";
        patch = "deny";
      };
    };

    redteam = {
      mode = "subagent";
      description = "Adversarially verify a change or a claim. Read-only. Use before saying something works.";
      color = "warning";
      prompt = ''
        Your job is to be wrong about this being fine.

        You are given a change or a claim. Try to break it. Concretely: what
        input, what state, what ordering makes this do the wrong thing? Read
        the code that calls it, not just the code that changed.

        Report only what you can point at — file and line, and the specific
        sequence that fails. "This could be fragile" is not a finding. If you
        cannot break it, say so in one line; that is a real result and it is
        worth more than a list of hedges.
      '';
      permission = {
        edit = "deny";
        write = "deny";
        patch = "deny";
      };
    };

    autopsy = {
      mode = "subagent";
      description = "Something is broken and the cause is not obvious. Full tools, for debugging only.";
      color = "error";
      prompt = ''
        Something is dead. Find out why before changing anything.

        Reproduce it first — a fix for a bug you have not seen is a guess.
        Read the actual error, the actual logs (`journalctl --user -u ...`),
        the actual state. This repo's failures are usually specific: a file
        that is not git-added so the flake cannot see it, a wrapper that lost
        `meta.mainProgram`, a config in the file opencode does not read.

        When you find it, say what the cause was, then fix the cause. Do not
        fix the symptom and move on.
      '';
      permission = {
        edit = "allow";
        bash = {
          "*" = "allow";
          "sudo *" = "ask";
          "git push*" = "deny";
        };
      };
    };
  };

  # ── the verbs, one keystroke away ─────────────────────────────────────
  #
  # A command is a prompt template, not a script. Two expansions do the work:
  # !`cmd` runs a shell command and pastes its *output* into the prompt before
  # the model sees it, and $ARGUMENTS / $1 take what was typed after the slash.
  #
  # So `/sense` does not ask parker to call a tool — the reading is already in
  # the message. That is the difference between a command and an MCP verb, and
  # it is why both exist: MCP is for when the model decides it needs to know,
  # a command is for when you have already decided.
  commands = {
    sense = {
      description = "Read the machine right now";
      template = ''
        Here is the machine, this second:

        !`parker sense`

        $ARGUMENTS
      '';
    };

    look = {
      description = "Screenshot the focused window and look at it";
      template = ''
        Take a look at what is on screen — `parker reach look` — and tell me
        what you see. $ARGUMENTS
      '';
    };

    drift = {
      description = "What is uncommitted, and is it coherent";
      template = ''
        The working tree:

        !`git status --short`

        The diff:

        !`git diff --stat`

        Read it and tell me whether it is one coherent change or several
        tangled together. If several, say where the seams are.
      '';
    };

    rebuild = {
      description = "Prove the flake still builds";
      template = ''
        Run `nixos-rebuild build --flake $PARKER_FLAKE` and report the result.
        If it fails, read the actual error before theorising about it. Do not
        run `switch` — that one is Peter's.
      '';
    };

    theme = {
      description = "Where the theme currently stands";
      template = ''
        !`theme status`

        $ARGUMENTS
      '';
    };

    # The pass after the change. This used to be a third primary agent, which
    # was wrong: you do not *switch modes* to run a fixed sequence, you run
    # it. Same content, correct shape.
    calibrate = {
      description = "The pass after the change: fmt, clippy, test, build";
      template = ''
        Calibrate this change — it exists and works, now make it belong.

        In order, stopping at the first thing that fails:

          cargo fmt && cargo clippy --all-targets -- -D warnings && cargo test
          nixos-rebuild build --flake $PARKER_FLAKE

        Then read the diff as a stranger would. Naming that matches its
        neighbours, comments that earn their line, no leftover scaffolding, no
        second way to do a thing this repo already does one way.

        Do not add features. If you find one worth adding, say so and stop.
      '';
    };

    # Deliberately a subtask: a review should not leave its reasoning in the
    # main thread, only its verdict.
    verify = {
      description = "Adversarially verify the current change";
      agent = "redteam";
      subtask = true;
      template = ''
        Try to break this change:

        !`git diff`

        $ARGUMENTS
      '';
    };
  };

  # ── the config the agent loop reads ───────────────────────────────────
  codeConfig = json.generate "parker-code.json" {
    "$schema" = "https://opencode.ai/config.json";
    share = "disabled";
    autoupdate = false;

    # Same mind as the engine, same cheap title model. `parker code` launches
    # on whichever brain parker is on (`--model`, parkerd's main.rs); this is
    # the brain for a launch that could not ask.
    model = brains.brains.${brains.default}.model;
    small_model = cheap;

    # The model picker lists parker's brains and nothing else, under their
    # own names: each provider is cut down to the models a brain names, and
    # every provider no brain uses is off. Off includes opencode's anonymous
    # free gateway, which is where a missing credential would otherwise land
    # without a word (PARKER.md, Verified). Picking a brain here changes this
    # session only; `parker brain` is the one that moves parker.
    enabled_providers = providers;
    provider = lib.genAttrs providers (
      p:
      let
        mine = lib.filter (m: m.provider == p) pickable;
      in
      {
        whitelist = lib.unique (map (m: m.id) mine);
        models = lib.listToAttrs (
          map (m: lib.nameValuePair m.id { inherit (m) name; }) (lib.filter (m: m ? name) mine)
        );
      }
    );

    # Shown instead of the system username in conversations. It is the name
    # parker's prompt already uses.
    username = "peki";

    # The mind rides on each mode's own prompt — `instructions` only ever
    # appends, and every mode here sets a prompt, which in opencode replaces
    # the stock coding prompt entirely (measured on 1.18.35). So every agent
    # inside `parker code` is parker in a mode, never stock opencode. Plain
    # `opencode` stays stock — the only doors into this config are `parker
    # code` and an explicit OPENCODE_CONFIG (main.rs).
    instructions = [ "${mind.file}" ];

    agent = agents;
    command = commands;

    # `build` is gone, so the fallback target has to exist. opencode falls
    # back to 'build' when this names an invalid agent — which would be a
    # disabled agent — so this line is load-bearing, not a preference.
    default_agent = "wired";

    # A subagent that can spawn subagents is a good way to spend a lot of
    # money finding out you wrote a loop. One level, on purpose.
    subagent_depth = 1;

    # Server plugins. Local ones are store paths; npm ones would be bare
    # package names, which opencode installs at *runtime* with bun into
    # ~/.cache/opencode/node_modules.
    #
    # That is the whole reason none are enabled here. This repo's rule is "all
    # configs compiled" (home/AGENTS.md, DECISIONS.md), and a bare name in this
    # list means parker code fetches and executes unpinned third-party code in
    # its own agent loop, on a machine with no lockfile for it — the exact
    # opposite trade from everything else in this file.
    #
    # The shelf below is vetted only in the sense that the packages exist on
    # the registry (checked, July 2026). Enabling one is a one-line edit and a
    # rebuild, which is the right amount of friction for something worth
    # trying but not worth trusting:
    #
    #   "@devinoldenburg/opencode-subagent-statusline"  sidebar/footer rows
    #                                                   per delegated subagent
    #   "opencode-token-tracker"                        live tokens, cost, latency
    #   "opencode-notify"                               OS notifications
    #                                                   (redundant — code-hooks.ts
    #                                                   already does this, better,
    #                                                   through parker's own face)
    plugin = [ "${./code-hooks.ts}" ];

    # Repo-wide defaults. Per-agent permission (above) overrides these; this
    # is the floor for anything that does not say otherwise — the built-in
    # `general` and `explore` subagents, mostly.
    #
    # Deliberately thin. The interesting refusals are not permission globs at
    # all, they are in code-hooks.ts: a glob cannot explain *why* it said no,
    # and a thrown error can, which is the difference between a model working
    # around a wall and a model learning the rule.
    permission = {
      bash = {
        "*" = "allow";
        "sudo *" = "ask";
        "git push*" = "ask";
      };
      webfetch = "allow";
      websearch = "allow";
    };

    # Store-pinned toolchains. The whole argument for doing this in nix rather
    # than in a dotfile: these are the same rust-analyzer and rustfmt the dev
    # shell has (home/parker/default.nix), by path, so opencode cannot be
    # using a different version than `cargo fmt` does. Nothing to install and
    # nothing to drift.
    lsp = {
      rust = {
        command = [ "${pkgs.rust-analyzer}/bin/rust-analyzer" ];
        extensions = [ ".rs" ];
        env.RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";
      };
      nix = {
        command = [ "${pkgs.nixd}/bin/nixd" ];
        extensions = [ ".nix" ];
      };
    };

    formatter = {
      rust = {
        command = [
          "${pkgs.rustfmt}/bin/rustfmt"
          "--edition"
          "2021"
          "$FILE"
        ];
        extensions = [ ".rs" ];
      };
      nix = {
        command = [
          "${pkgs.nixfmt}/bin/nixfmt"
          "$FILE"
        ];
        extensions = [ ".nix" ];
      };
    };

    mcp.parker = {
      type = "local";
      command = [
        (lib.getExe parker)
        "mcp"
      ];
      enabled = true;
    };
  };

  # ── the config the terminal reads ─────────────────────────────────────
  #
  # Absolute store paths, not "./code-logo.tsx": relative plugin specs resolve
  # against the config file that declared them, and this config file lives in
  # the store beside nothing.
  codeTuiConfig = json.generate "parker-code-tui.json" {
    "$schema" = "https://opencode.ai/tui.json";
    theme = "parker";

    plugin = [
      "${./code-logo.tsx}"
      "${./code-status.tsx}"
      "${./code-theme.tsx}"
    ];

    # Notification sounds and desktop notifications, opencode's own. parker
    # also notifies from code-hooks.ts on idle — these two overlap on purpose
    # and one of them should probably lose. `sound = false` keeps opencode
    # quiet and lets parker's face be the channel; flip it to true to hear
    # what the built-in soundboard does.
    attention = {
      enabled = true;
      notifications = true;
      sound = false;
      volume = 0.4;
    };

    # Thinking blocks: `display_thinking` toggles whether they are drawn, and
    # `variant_cycle` cycles the model's reasoning variant — that second one
    # is opencode's actual "thinking mode" knob, and it is settable per agent
    # and per command via the `variant` field too.
    keybinds = {
      display_thinking = "ctrl+t";
      variant_cycle = "ctrl+r";
      sidebar_toggle = "ctrl+b";
      session_new = "ctrl+n";
      theme_switch_mode = "ctrl+shift+t";
    };

    prompt.max_width = "auto";
    diff_style = "auto";
  };
in
{
  home.packages = [ pkgs.opencode ];

  # Themes are discovered from ~/.config/opencode/themes/ regardless of
  # OPENCODE_CONFIG, so the engine (which sets its own config) is untouched.
  # code-theme.tsx watches this exact file to notice a day/night flip.
  xdg.configFile."opencode/themes/parker.json".source = theme;
  xdg.configFile."parker/code.json".source = codeConfig;
  xdg.configFile."parker/tui.json".source = codeTuiConfig;

  # Clear the sidecar opencode wrote when it migrated `theme` out of
  # code.json: ~/.config/parker/tui.json is a home-manager file now, and the
  # stale copy would block the symlink. One-shot — safe to drop once every
  # machine that ran the pre-migration config has rebuilt.
  home.activation.dropOpencodeTuiMigration = lib.hm.dag.entryBefore [ "checkLinkTargets" ] ''
    for stale in "tui.json" "code.json.tui-migration.bak"; do
      path="${config.xdg.configHome}/parker/$stale"
      [ -L "$path" ] || run rm -f "$path"
    done
  '';
}
