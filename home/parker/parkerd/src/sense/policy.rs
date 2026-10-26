// Privacy. Window titles are the most revealing thing parker can see, and
// this is the layer that decides what it is allowed to keep.
//
// v0 is deliberately a floor, not a solution. Two rules:
//
//   1. Some applications never have their titles read at all (class match).
//   2. Some words in a title redact it wherever it appears (substring match).
//
// The honest limitation, worth stating rather than hiding: in a browser the
// title *is* the page, so rule 2 fails open — a sensitive site nobody listed
// still gets through. The real fix is not a longer word list; it is that
// parker looks at a screen only when asked (the `look` verb, later), so
// continuous sensing never needs to be trusted with much. Until then, keep
// this list short and assume it is porous.
//
// No config file in v0 — these are compiled in, which means changing them is
// a diff, which is the property the rest of this repo already has.

use super::Focus;

/// Classes whose titles are never read. Matched case-insensitively as a
/// prefix, so a bundle id covers its variants.
const DENY_CLASS: &[&str] = &[
    "org.keepassxc",
    "keepassxc",
    "1password",
    "bitwarden",
    "com.bitwarden",
    "org.gnome.seahorse",
];

/// Words that redact any title containing them, in any application.
/// Case-insensitive substring match.
const DENY_TITLE: &[&str] = &[
    "password",
    "passwd",
    "incognito",
    "private browsing",
    "bank",
    "paypal",
    "authenticator",
    "2fa",
    "secret",
    "recovery phrase",
    "seed phrase",
];

pub const REDACTED: &str = "<redacted>";

/// Replace the title in place when policy says so. The class always survives
/// — knowing you are in a password manager is not sensitive, and parker needs
/// it to know to leave you alone.
pub fn redact(focus: &mut Focus) {
    if should_redact(&focus.class, &focus.title) {
        focus.title = REDACTED.to_string();
        focus.redacted = true;
    }
}

/// Class-only check, for `reach::look`. Deliberately narrower than
/// `should_redact`: a screenshot is an explicit act the user asked for, so it
/// is refused only for applications that are categorically never captured,
/// never on a keyword that happened to appear in a title.
pub fn class_denied(class: &str) -> bool {
    let class_lc = class.to_lowercase();
    DENY_CLASS.iter().any(|d| class_lc.starts_with(d))
}

fn should_redact(class: &str, title: &str) -> bool {
    if class_denied(class) {
        return true;
    }
    let title_lc = title.to_lowercase();
    DENY_TITLE.iter().any(|d| title_lc.contains(d))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn focus(class: &str, title: &str) -> Focus {
        Focus {
            class: class.into(),
            title: title.into(),
            workspace: 1,
            workspace_name: "1".into(),
            fullscreen: 0,
            floating: false,
            redacted: false,
            held_secs: None,
        }
    }

    #[test]
    fn ordinary_titles_pass_through() {
        let mut f = focus("com.mitchellh.ghostty", "parker | ~");
        redact(&mut f);
        assert_eq!(f.title, "parker | ~");
        assert!(!f.redacted);
    }

    #[test]
    fn denied_class_loses_its_title_but_keeps_its_class() {
        let mut f = focus("org.keepassxc.KeePassXC", "Personal.kdbx");
        redact(&mut f);
        assert_eq!(f.title, REDACTED);
        assert!(f.redacted);
        assert_eq!(f.class, "org.keepassxc.KeePassXC");
    }

    #[test]
    fn class_match_is_case_insensitive() {
        let mut f = focus("1Password", "Vault");
        redact(&mut f);
        assert!(f.redacted);
    }

    #[test]
    fn denied_word_redacts_any_application() {
        let mut f = focus("google-chrome", "My Bank — Accounts");
        redact(&mut f);
        assert_eq!(f.title, REDACTED);
        assert!(f.redacted);
    }

    #[test]
    fn redaction_is_flagged_not_silent() {
        // parker must be able to tell "no title" from "title withheld".
        let mut f = focus("google-chrome", "reset your password");
        redact(&mut f);
        assert!(f.redacted);
        assert_ne!(f.title, "");
    }

    #[test]
    fn known_gap_a_sensitive_site_nobody_listed_gets_through() {
        // Documented, not accepted: this is why `look` is on-demand.
        let mut f = focus("google-chrome", "Nykredit — Konto");
        redact(&mut f);
        assert!(!f.redacted);
    }
}
