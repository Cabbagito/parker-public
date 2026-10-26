# parker's mind — the prompt every parker agent wears.
#
# Four files, stitched in a fixed order at build time (PARKER.md,
# Personality). Split by who may edit them, which is the reason for the
# split at all:
#
#   identity.md   who parker is — ours; parker never edits it
#   peter.md      who Peter is — ours too, and its last section dates
#   voice.md      how he speaks, and the face as a channel
#   hands.md      how he works — batching, tool choice, building
#
# `beliefs.md` (parker-writable) is the planned fourth and is not here yet;
# when it lands it is a live path in `instructions`, like AGENTS.md, not a
# part of this build-time text.
#
# Both consumers import this, so they cannot drift apart: engine.nix uses
# `text` as the parker agent's whole prompt, and code.nix appends `file` to
# every mode through `instructions`. Nothing here is discovered at run time
# — every part loads every time, or the build fails.
{ pkgs, lib }:
let
  parts = [
    ./mind/identity.md
    ./mind/peter.md
    ./mind/voice.md
    ./mind/hands.md
  ];

  # A missing file already fails evaluation; an emptied one would not, and
  # would ship a parker with a hole in him. Refuse that loudly.
  read =
    f:
    let
      t = builtins.readFile f;
    in
    assert lib.assertMsg (lib.strings.trim t != "") "parker's mind: ${baseNameOf f} is empty";
    lib.strings.trim t;

  text = lib.concatMapStringsSep "\n\n" read parts + "\n";
in
{
  inherit text;
  file = pkgs.writeText "parker-mind.md" text;
}
