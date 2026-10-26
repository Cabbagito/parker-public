// Which model parker thinks with — his brain.
//
// The set is home/parker/brains.nix (a name we chose → a record: the
// provider/model string opencode wants, and the lore it is named for),
// compiled to ~/.config/parker/brains.json, which is what this reads. The choice is one line in $XDG_STATE_HOME/parker/brain:
// state, not runtime, because the brain Peter picked last night is still the
// one he wants this morning. A face or a dance is a moment; this is not.
//
// Nothing here talks to the engine. A brain is applied per message, by
// whoever sends one: door (visage and the porthole) and parker-chat ask
// before every send, and `parker code` asks once at launch. So switching
// costs nothing — no rebuild, no restart — and this file is the one place a
// name becomes a model.
//
// Not a verb, on purpose: there is no `reach brain`, so parker cannot pick
// his own. Spending Peter's money on his own initiative is exactly what the
// gate in reach/mod.rs is for, and that gate arrives with the daemon. He
// knows which brain he is on — door says so with every message — and he can
// ask for another.

use crate::reach::Done;
use serde_json::Value;
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Brain {
    pub name: String,
    /// opencode's `provider/model`.
    pub model: String,
    /// The book on Peter's shelf it is named for, as visage's card says it.
    /// Optional: a brain without one is still a brain.
    pub lore: Option<String>,
}

#[derive(Debug)]
pub struct Catalog {
    /// In name order — brains.json is an object, and serde_json keeps its
    /// keys sorted.
    pub brains: Vec<Brain>,
    /// An index into `brains`, valid by construction (`parse`).
    default: usize,
}

impl Catalog {
    /// brains.json, or nothing. A file that names a default it does not
    /// list is refused whole: a catalog with no fallback could not answer
    /// "which brain?" for a choice that has gone stale.
    pub fn parse(raw: &str) -> Option<Catalog> {
        let v: Value = serde_json::from_str(raw).ok()?;
        let brains: Vec<Brain> = v
            .get("brains")?
            .as_object()?
            .iter()
            .filter_map(|(name, brain)| {
                Some(Brain {
                    name: name.clone(),
                    model: brain.get("model")?.as_str()?.to_string(),
                    lore: brain
                        .get("lore")
                        .and_then(Value::as_str)
                        .map(str::to_string),
                })
            })
            .collect();
        let default_name = v.get("default")?.as_str()?;
        let default = brains.iter().position(|b| b.name == default_name)?;
        Some(Catalog { brains, default })
    }

    pub fn default(&self) -> &Brain {
        &self.brains[self.default]
    }

    /// The brain in use: the one chosen, while it is still listed, else the
    /// default. Renaming a brain in brains.nix therefore never strands a
    /// choice — it lands back on the default.
    pub fn current(&self, chosen: Option<&str>) -> &Brain {
        chosen
            .and_then(|c| self.brains.iter().find(|b| b.name == c))
            .unwrap_or_else(|| self.default())
    }

    /// A brain from what someone typed. The full name, its initials (`jrti`
    /// — the name is a mouthful), enough of its start, or a piece of its
    /// model (`opus`, `haiku`) — whichever finds exactly one. Two matches is
    /// a question back, never a guess.
    pub fn find(&self, asked: &str) -> Result<&Brain, String> {
        let want = asked
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .to_lowercase();
        if want.is_empty() {
            return Err(format!("which brain? {}", self.menu()));
        }
        let tries: [&dyn Fn(&Brain) -> bool; 4] = [
            &|b| b.name.to_lowercase() == want,
            &|b| b.name.contains(' ') && initials(&b.name) == want,
            &|b| b.name.to_lowercase().starts_with(&want),
            &|b| b.model.to_lowercase().contains(&want),
        ];
        for matches in tries {
            let hits: Vec<&Brain> = self.brains.iter().filter(|b| matches(b)).collect();
            match hits.as_slice() {
                [] => continue,
                [one] => return Ok(one),
                many => {
                    let names: Vec<&str> = many.iter().map(|b| b.name.as_str()).collect();
                    return Err(format!("'{asked}' could be {}", names.join(" or ")));
                }
            }
        }
        Err(format!("no brain '{asked}'. {}", self.menu()))
    }

    fn menu(&self) -> String {
        let names: Vec<&str> = self.brains.iter().map(|b| b.name.as_str()).collect();
        format!("brains: {}", names.join(", "))
    }
}

fn initials(name: &str) -> String {
    name.split_whitespace()
        .filter_map(|w| w.chars().next())
        .collect::<String>()
        .to_lowercase()
}

fn under(var: &str, fallback: &str, file: &str) -> Option<PathBuf> {
    let base = std::env::var_os(var)
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(fallback)))?;
    Some(base.join("parker").join(file))
}

fn catalog_file() -> Option<PathBuf> {
    under("XDG_CONFIG_HOME", ".config", "brains.json")
}

fn choice_file() -> Option<PathBuf> {
    under("XDG_STATE_HOME", ".local/state", "brain")
}

pub fn catalog() -> Result<Catalog, String> {
    let path = catalog_file().ok_or("no home to find brains.json in")?;
    let raw = std::fs::read_to_string(&path)
        .map_err(|e| format!("no brains: cannot read {}: {e}", path.display()))?;
    Catalog::parse(&raw).ok_or_else(|| format!("no brains: {} is not a brain list", path.display()))
}

fn chosen() -> Option<String> {
    let raw = std::fs::read_to_string(choice_file()?).ok()?;
    Some(raw.trim().to_string()).filter(|s| !s.is_empty())
}

/// The brain in use right now, and the set it came from.
pub fn current() -> Result<(Brain, Catalog), String> {
    let cat = catalog()?;
    let now = cat.current(chosen().as_deref()).clone();
    Ok((now, cat))
}

// ── what `parker brain` says ────────────────────────────────────────────────

/// No name: which brain, and what it is.
pub fn show() -> Done {
    let (now, _) = current()?;
    Ok(format!("{} ({})", now.name, now.model))
}

/// Only the model string — what parker-chat and `parker code` hand opencode.
pub fn model() -> Done {
    Ok(current()?.0.model)
}

/// Every brain, the one in use marked.
pub fn list() -> Done {
    let (now, cat) = current()?;
    let width = cat.brains.iter().map(|b| b.name.len()).max().unwrap_or(0);
    let lines: Vec<String> = cat
        .brains
        .iter()
        .map(|b| {
            let mark = if *b == now { '*' } else { ' ' };
            let default = if b.name == cat.default().name {
                "  (default)"
            } else {
                ""
            };
            format!("{mark} {:width$}  {}{default}", b.name, b.model)
        })
        .collect();
    Ok(lines.join("\n"))
}

/// The whole picture, for door: the brain in use, the default, and the set.
pub fn json() -> Done {
    let (now, cat) = current()?;
    let brains: Vec<Value> = cat
        .brains
        .iter()
        .map(|b| serde_json::json!({ "name": b.name, "model": b.model, "lore": b.lore }))
        .collect();
    Ok(serde_json::json!({
        "name": now.name,
        "model": now.model,
        "default": cat.default().name,
        "brains": brains,
    })
    .to_string())
}

/// Pick a brain. It applies from the next message on; a turn already
/// running finishes on the brain it started with.
pub fn set(asked: &str) -> Done {
    let cat = catalog()?;
    let brain = cat.find(asked)?.clone();
    if cat.current(chosen().as_deref()) == &brain {
        return Ok(format!("already {} ({})", brain.name, brain.model));
    }
    let path = choice_file().ok_or("no home to keep the choice in")?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("could not pick a brain: {e}"))?;
    }
    std::fs::write(&path, format!("{}\n", brain.name))
        .map_err(|e| format!("could not pick a brain: {e}"))?;
    Ok(format!(
        "brain: {} ({}), from the next message on",
        brain.name, brain.model
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    const FOUR: &str = r#"{
        "default": "deep thought",
        "brains": {
            "deep thought": { "model": "anthropic/claude-opus-5-5", "lore": "Adams" },
            "just read the instructions": { "model": "anthropic/claude-sonnet-5-5" },
            "speedy": { "model": "anthropic/claude-haiku-5-5", "lore": "Asimov" },
            "wintermute": { "model": "anthropic/claude-fable-5-1" }
        }
    }"#;

    fn four() -> Catalog {
        Catalog::parse(FOUR).expect("the fixture parses")
    }

    fn found(cat: &Catalog, asked: &str) -> String {
        cat.find(asked)
            .unwrap_or_else(|e| panic!("{asked:?}: {e}"))
            .name
            .clone()
    }

    #[test]
    fn every_way_of_saying_a_brain_finds_it() {
        let cat = four();
        for (asked, name) in [
            ("deep thought", "deep thought"),
            ("  Deep   Thought ", "deep thought"),
            ("jrti", "just read the instructions"),
            ("dt", "deep thought"),
            ("just", "just read the instructions"),
            ("speedy", "speedy"),
            ("winter", "wintermute"),
            ("opus", "deep thought"),
            ("sonnet", "just read the instructions"),
            ("haiku", "speedy"),
            ("fable", "wintermute"),
            ("anthropic/claude-haiku-5-5", "speedy"),
        ] {
            assert_eq!(found(&cat, asked), name, "{asked:?}");
        }
    }

    #[test]
    fn two_matches_is_a_question_not_a_guess() {
        // Every model is anthropic's, so this names all four.
        let err = four().find("anthropic").unwrap_err();
        assert!(err.contains("could be"), "{err}");
    }

    #[test]
    fn an_unknown_brain_is_refused_and_says_what_there_is() {
        let cat = four();
        for asked in ["gpt", "5head", ""] {
            let err = cat.find(asked).unwrap_err();
            assert!(
                err.contains("speedy"),
                "{asked:?} did not list the brains: {err}"
            );
        }
    }

    #[test]
    fn a_stale_choice_falls_back_to_the_default() {
        let cat = four();
        assert_eq!(cat.current(Some("speedy")).name, "speedy");
        assert_eq!(cat.current(Some("regular joe")).name, "deep thought");
        assert_eq!(cat.current(None).name, "deep thought");
    }

    #[test]
    fn a_catalog_without_its_default_is_no_catalog() {
        assert!(
            Catalog::parse(r#"{"default": "nope", "brains": {"speedy": {"model": "a/b"}}}"#)
                .is_none()
        );
        assert!(Catalog::parse(r#"{"brains": {"speedy": {"model": "a/b"}}}"#).is_none());
        assert!(Catalog::parse("not json").is_none());
    }

    #[test]
    fn a_brain_is_a_record_and_its_lore_is_optional() {
        let cat = four();
        assert_eq!(cat.default().model, "anthropic/claude-opus-5-5");
        assert_eq!(cat.default().lore.as_deref(), Some("Adams"));
        assert_eq!(cat.find("jrti").map(|b| b.lore.clone()), Ok(None));
        // A bare model string is not a brain: a record has the `model` to
        // read, and a set left without its default is no set.
        assert!(Catalog::parse(r#"{"default": "speedy", "brains": {"speedy": "a/b"}}"#).is_none());
    }
}
