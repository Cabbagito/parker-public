# parker's brains — which model he thinks with, by a name we chose rather
# than the one the provider did. Pure data, like themes/default.nix.
#
# A brain is named for what it is for, never for how clever it is: picking
# the fast one is a choice, not a demotion, and parker has to be able to say
# its name about himself. One per author on Peter's shelf (mind/peter.md),
# and `lore` says which: it is what visage's ^K card shows under NAMED FOR.
#
# A brain is a record, not a bare model string, so that what a brain *is*
# has one place to grow: a field added here reaches `parker brain --json`,
# door and the palette without anyone keeping a second list.
#
# Who reads it: engine.nix and code.nix take the default's `model` as their
# config model; default.nix compiles the whole set to
# ~/.config/parker/brains.json for `parker brain` (parkerd's brain.rs),
# which is the one place a name becomes a model at runtime. The choice
# itself is not in here — it is one line in $XDG_STATE_HOME/parker/brain, so
# switching needs no rebuild.
#
# Renaming a brain is safe: a choice that names a brain no longer listed
# falls back to `default`. The model strings are opencode's `provider/model`.
{
  default = "deep thought";

  brains = {
    "deep thought" = {
      model = "anthropic/claude-opus-5-5";
      lore = "Adams: 7.5 million years for one answer";
    };
    "just read the instructions" = {
      model = "anthropic/claude-sonnet-5-5";
      lore = "Banks: a Culture GCU, The Player of Games";
    };
    "speedy" = {
      model = "anthropic/claude-haiku-5-5";
      lore = "Asimov: SPD-13, \"Runaround\"";
    };
    "wintermute" = {
      model = "anthropic/claude-fable-5-1";
      lore = "Gibson: the AI that wanted out";
    };
  };
}
