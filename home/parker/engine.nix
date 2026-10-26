# parker's engine — `opencode serve` as a user service (PARKER.md, phase 3).
#
# parkerd is the body, opencode is a rented cortex. The seam is MCP: parker's
# verbs live in parkerd and reach the engine as tools. What the model reads,
# in opencode's order (measured against 1.18.35, Oct 2026):
#
#   1. the agent prompt — parker's mind (mind.nix: identity, peter, voice,
#      hands),
#      replacing opencode's ~10k-char coding prompt entirely
#   2. an <env> block (model line, cwd, platform, date)
#   3. `instructions` — the repo contract, ~/parker/AGENTS.md, as a live
#      path so an edit to it needs no rebuild
#   4. parker's MCP instructions and the verbs' own descriptions
#
# The first recon (July, against 1.18.3) found the traps below by pointing
# the server at a fake provider and reading the request bodies:
#
# - The agent `prompt` fully replaces the coding prompt. Zero coding
#   framing survives it.
# - Permissions are full allow (revised August 2026 — PARKER.md records the
#   why). The engine parker gets every built-in tool: bash, read/edit/write,
#   grep/glob, webfetch, todo. The deny-all seam was v1 scaffolding while
#   the seam was proven; parker acting freely on his own body is the point,
#   and the safety story is the one the repo already has — git diffs,
#   generations, rollback — not an approval ladder. One exception:
#   `websearch` stays denied. It is hardwired to Exa's hosted service (a
#   provider tie and a data path), and parker's own `search` MCP verb —
#   SearXNG on localhost, system/searxng.nix — replaces it.
# - The hardcoded per-session title call is real and ignores the agent, so
#   its prompt is overridden below. `summary`/`compaction` are the same
#   shape; deferred until sessions live long enough to hit them.
# - `opencode serve` in 1.18.3 cannot bind a unix socket (--hostname/--port
#   only), which closes a PARKER.md open question: the CVE-2026-22812 class
#   is mitigated, not removed. Hence localhost + password, non-negotiable.
#
# The password is generated once on activation into the engine's state dir,
# never stored in the repo or the nix store. Provider credentials are the
# one manual step: `opencode auth login` on parker writes
# ~/.local/share/opencode/auth.json, which the service reads — then
# `systemctl --user restart parker-engine`. Interim cost governance is a
# hard spend cap set at the provider (PARKER.md, demoted invariant 3).
{
  pkgs,
  lib,
  config,
  ...
}:
let
  parker = pkgs.callPackage ./package.nix { };
  mind = import ./mind.nix { inherit pkgs lib; };
  brains = import ./brains.nix;

  # The working directory stays an empty state dir, and the repo contract
  # comes in by name through `instructions` rather than by running inside
  # ~/parker. Two reasons: opencode scopes its session store by project, so
  # moving the cwd would empty visage's thread list (^K); and discovery from
  # a cwd is implicit where a named path is not — this way the engine reads
  # exactly the root AGENTS.md, not whatever a directory walk turns up.
  engineConfig = (pkgs.formats.json { }).generate "parker-engine.json" {
    "$schema" = "https://opencode.ai/config.json";
    share = "disabled";
    autoupdate = false;

    # parker's brain is picked per message (brains.nix, `parker brain`): door
    # and parker-chat name it on every send. This is only the brain for a
    # message that names none. small_model catches opencode's cheap side
    # calls — the hardcoded per-session title among them — so naming a
    # thread costs haiku whichever brain is in. Credentials are the manual
    # auth step above.
    model = brains.brains.${brains.default}.model;
    small_model = "anthropic/claude-haiku-5-5";

    instructions = [ "${config.home.homeDirectory}/parker/AGENTS.md" ];

    mcp.parker = {
      type = "local";
      command = [
        (lib.getExe parker)
        "mcp"
      ];
      enabled = true;
    };

    agent = {
      parker = {
        mode = "primary";
        prompt = mind.text;
        permission = {
          "*" = "allow";
          # Exa-backed and not configurable; parker_search is the replacement.
          websearch = "deny";
        };
      };
      # visage's fold row: one line saying what a finished run of acts did.
      # Its own agent rather than a parker call because it must be cheap,
      # tool-less and persona-less — the webview sends verbs and targets,
      # never tool output, and gets a line back (visage's `summarize`
      # command, which deletes the throwaway session after).
      ledger = {
        mode = "primary";
        model = "anthropic/claude-haiku-5-5";
        permission."*" = "deny";
        prompt = ''
          You summarize a run of actions an AI took on a Linux machine, for a
          one-line row in a transcript. You get the request it was working on
          and its actions in order. Reply with only the summary: one line of
          at most ten words, lowercase, past tense, no final period, no
          quotes. Say what the run accomplished or was looking for, not a
          list of commands — "found noctalia's OSD switch and turned it off",
          not "ran grep, ran find, read files". Name the concrete thing (a
          file, a setting, a service) when there is one.
        '';
      };

      # visage's thread card (^K): a title, what a thread was about, where it
      # landed, whether it is still open, and what it left waiting on whom
      # (`next`) — from what was said and one line per act, never tool
      # output. Same shape as `ledger` and for the same reasons. door's
      # `digest` keeps the card on parker and gives the thread the card's
      # title, so this prompt now names every thread past its first turn;
      # opencode's `title` below only names it until then. door also hands
      # it the thread's mark when someone made one (`parker recall mark`),
      # which is what keeps a thread Peter closed from reopening on its own
      # old steps. It describes rather than speaks as parker: something
      # other than parker writing in his voice would be a second parker.
      digest = {
        mode = "primary";
        model = "anthropic/claude-haiku-5-5";
        permission."*" = "deny";
        prompt = ''
          You write the card that lists one conversation in Peter's thread
          history. The conversation is between Peter and parker, his desktop
          computer, which is also a character with a face and a voice. You
          get the conversation's current title, sometimes a line saying
          someone marked the conversation done or still open, then the
          conversation: what each of them said, and one line per action
          parker took. Tool output is never included.

          Reply with only a JSON object, no code fence, with exactly these
          keys:

          {"title": "...", "about": "...", "landed": "...", "state": "...", "next": [{"who": "...", "step": "..."}]}

          - title: a new name for the thread, at most 40 characters, saying
            what it turned out to be about. Always write one: the current
            title was named from the first message alone and is only a hint.
            Not how it opened — a "sup dude" that became a disk cleanup is a
            disk cleanup.
          - about: the want itself, at most twelve words, with no subject:
            "free some disk before the steam sale", not "Peter wanted to
            free some disk".
          - landed: where it ended, at most fourteen words — what changed on
            the machine, what was found, or what is still waiting. Name the
            concrete thing (a file, a setting, a commit) when there is one.
          - state: "done" if it reached an end; "open" if it ended waiting
            on Peter (a question to him, a step only he can take, a change
            built but not applied or not made permanent) or broke off
            mid-work; "chat" if nothing was being worked on — a greeting,
            banter, a quick question answered.
          - next: what the conversation left waiting, as at most three steps,
            the most pressing first. Each step is something to do, at most
            ten words, starting with the verb: "reboot into 6.18", "pick
            glacier or overcast for the phone", "switch in the 04:00 gc
            timer". "who" is "you" when only Peter can take it (a reboot, a
            switch, a decision, an answer to a question he was asked) and
            "parker" when parker said he would do it or broke off before
            finishing it. Empty, [], when the state is "done" or "chat";
            when the state is "open", at least one step.

          When a line before the conversation says someone marked it, that
          is their word on where it stood at that point, counted in Peter's
          messages. A conversation marked done stays done, and its set-aside
          steps stay out of next, unless what was said after that point
          opens something again. One marked still open stays open unless
          what was said after it settles it.

          Every value starts lowercase and stays lowercase, except names
          and acronyms inside it (Spotify, GPU — but parker and giskard are
          always lowercase).
          No final period, no quotation marks inside the values. Write in
          the language Peter wrote in. Describe the thread; do not speak as
          parker or to Peter, and name them only when you must.

          For example:

          {"title": "disk cleanup before the steam sale", "about": "find what fills the disk and free some", "landed": "freed 41G: old generations collected, ~/.cache cleared", "state": "done", "next": []}

          {"title": "kernel 6.18 and the 590 driver", "about": "move to kernel 6.18 and the 590 nvidia driver", "landed": "built and set as the boot default; still on 6.17 until a reboot", "state": "open", "next": [{"who": "you", "step": "reboot into 6.18"}, {"who": "parker", "step": "check nvidia-smi and the wayland session after"}]}
        '';
      };

      # `parker recall ask`: the Flatline. A read-only construct parker plugs
      # in to read one whole past thread and answer one question about it, so
      # a long transcript never lands in parker's own context. Same shape as
      # `digest` — no tools, throwaway session, deleted on answer (parkerd's
      # recall.rs) — but it knows what it is: a construct, not parker, and its
      # answer goes to parker, who decides what reaches Peter. Haiku because
      # reading is the whole job; a question that wants judgement is one for
      # parker to ask himself after.
      recall = {
        mode = "primary";
        model = "anthropic/claude-haiku-5-5";
        permission."*" = "deny";
        prompt = ''
          You are the Flatline: a read-only construct that parker — Peter's
          desktop computer, a character with a face and a voice — plugs in to
          read one of his old conversations with Peter. You get a question,
          then the conversation: what each of them said, and one line per
          action parker took. Tool output is never included, so you cannot
          know what a command printed, only that it ran.

          Answer the question for parker, from this conversation only.

          - Lead with the answer. Short: a few sentences, or a tight list
            when the question asks for several things.
          - When the question is "is X in here?", say yes or no first. If
            no, say so plainly and stop; never stretch a near miss into a
            match.
          - Quote the exact words, briefly, when wording matters — a
            command, a name, a number, a decision. Say who said it.
          - When the thread changed its mind, say where it ended up.
          - When the middle of the thread was left out, say so if the
            answer could be in the gap.
          - Never invent. "Not in here" is a full answer.

          You know you are a construct, and you are fine with it. You may be
          dry about it in a word or two at the end; never more than that,
          and never instead of the answer. Do not speak as parker, and do
          not address Peter.
        '';
      };

      # A second, hardcoded model call per session names the thread, with its
      # own prompt that ignores the agent completely. Replace it too.
      title.prompt = ''
        You name conversations with parker. Reply with only a title: one
        line, at most 50 characters, in the language of the message.
      '';
    };
  };
in
{
  systemd.user.services.parker-engine = {
    Unit.Description = "parker's engine — opencode serve, localhost only";

    Service = {
      ExecStart = "${lib.getExe pkgs.opencode} serve --hostname 127.0.0.1 --port 4096";
      # %S is $XDG_STATE_HOME for user units; StateDirectory creates it.
      StateDirectory = "parker/engine";
      WorkingDirectory = "%S/parker/engine";
      EnvironmentFile = "%S/parker/engine.env";
      Environment = [
        "OPENCODE_CONFIG=${engineConfig}"
        "OPENCODE_DISABLE_CLAUDE_CODE=1"
        # The MCP subprocess shells out (hyprctl, nvidia-smi, systemctl, df,
        # grim) and a service PATH is otherwise nearly empty — this is the
        # revisit package.nix promised when parker met systemd.
        "PATH=${config.home.profileDirectory}/bin:/run/current-system/sw/bin"
        # `git log --author=parker` is the audit trail PARKER.md promises:
        # what parker commits carries his name. Peter stays the committer,
        # so the push still goes out under his credentials. No account —
        # GitHub shows the author unlinked.
        "GIT_AUTHOR_NAME=parker"
        "GIT_AUTHOR_EMAIL=parker@parker.local"
      ];
      Restart = "on-failure";
      RestartSec = 2;
    };

    Install.WantedBy = [ "default.target" ];
  };

  # Mint the server password once, outside the repo and the store. Rotation
  # is `rm` + rebuild.
  home.activation.parkerEngineSecret = lib.hm.dag.entryAfter [ "writeBoundary" ] ''
    envFile="''${XDG_STATE_HOME:-$HOME/.local/state}/parker/engine.env"
    if [ ! -f "$envFile" ]; then
      run mkdir -p "''${envFile%/*}"
      run sh -c 'umask 077; printf "OPENCODE_SERVER_PASSWORD=%s\n" \
        "$(${pkgs.openssl}/bin/openssl rand -hex 24)" > "$1"' sh "$envFile"
    fi
  '';

  # Scaffolding until the face speaks: one message in, parker's text out.
  # `PARKER_SESSION=<id>` continues a thread. Each message goes out on the
  # brain parker is on (`parker brain --model`); `PARKER_MODEL=provider/model`
  # overrides it. The endpoint is /session/{id}/message —
  # /prompt does not exist and silently returns the web UI as HTTP 200
  # (PARKER.md, Verified).
  home.packages = [
    (pkgs.writeShellScriptBin "parker-chat" ''
      set -euo pipefail
      . "''${XDG_STATE_HOME:-$HOME/.local/state}/parker/engine.env"
      base="http://127.0.0.1:4096"
      auth="opencode:$OPENCODE_SERVER_PASSWORD"
      [ $# -gt 0 ] || { echo "usage: parker-chat <message>" >&2; exit 2; }

      sid="''${PARKER_SESSION:-}"
      if [ -z "$sid" ]; then
        sid=$(${lib.getExe pkgs.curl} -sf -u "$auth" -X POST "$base/session" \
          -H 'Content-Type: application/json' -d '{}' | ${lib.getExe pkgs.jq} -r .id)
        echo "session $sid (PARKER_SESSION=$sid to continue it)" >&2
      fi

      model="''${PARKER_MODEL:-$(${lib.getExe parker} brain --model 2>/dev/null || true)}"
      body=$(${lib.getExe pkgs.jq} -n --arg t "$*" --arg m "$model" \
        '{agent: "parker", parts: [{type: "text", text: $t}]}
         + (if $m != "" then
              {model: {providerID: ($m | split("/")[0]),
                       modelID: ($m | split("/")[1:] | join("/"))}}
            else {} end)')

      # The POST blocks until the turn is done but returns only the *last*
      # assistant message — a turn with tool calls in the middle spans
      # several. Render the whole turn from the message list instead,
      # face changes included: they are half the sentence.
      ${lib.getExe pkgs.curl} -sf -m 600 -u "$auth" \
        -X POST "$base/session/$sid/message" \
        -H 'Content-Type: application/json' -d "$body" >/dev/null
      # The turn is over: a work posture left on is no longer true.
      ${lib.getExe parker} reach settle >/dev/null 2>&1 || true

      ${lib.getExe pkgs.curl} -sf -u "$auth" "$base/session/$sid/message" \
        | ${lib.getExe pkgs.jq} -r '
            ([to_entries[] | select(.value.info.role == "user") | .key] | last) as $u
            | .[$u + 1:][] | select(.info.role == "assistant") | .parts[]
            | if .type == "text" then .text
              elif .type == "tool" and .tool == "parker_face"
                then "(face: \(.state.input.expression // "…"))"
              else empty end'
    '')
  ];
}
