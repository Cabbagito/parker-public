// parker's markdown, from text to the HTML Message.svelte puts on screen.
//
// marked + DOMPurify is the mandatory path: the engine can quote the open
// web, and this webview holds Tauri IPC. Sanitizing is the last thing that
// touches the markup — everything after it (reveal, the caret) only
// removes nodes or adds our own element, never re-reads parker's text as
// HTML. Keep it that way in any new render path.
//
// What CSS cannot do on marked's own output happens here:
//   callouts  GitHub's `> [!NOTE]` … `> [!CAUTION]`, which marked prints
//             literally, become a blockquote with a class and a label
//   links     open beside visage, never in place of it: a click in the
//             porthole used to navigate the tab away from the face
//   reveal    typing counts *rendered* characters, so a bold is bold from
//             its first letter instead of four literal asterisks until it
//             closes, and a link is never raw brackets and a URL

import { marked } from 'marked';
import DOMPurify from 'dompurify';

const ALERT = /<blockquote>\s*<p>\[!(note|tip|important|warning|caution)\]\s*(<\/p>\s*<p>)?/gi;

DOMPurify.addHook('afterSanitizeAttributes', (node) => {
  if (node.tagName === 'A' && node.hasAttribute('href')) {
    node.setAttribute('target', '_blank');
    node.setAttribute('rel', 'noopener noreferrer');
  }
});

// A block's HTML is asked for twenty times a second while it types; the
// text under it does not change, so neither does the answer.
const cache = new Map();
const CACHE_CAP = 64;

/// Markdown → sanitized HTML, and how many characters of it are visible.
export function render(text) {
  let hit = cache.get(text);
  if (hit) return hit;
  const raw = marked.parse(text).replace(
    ALERT,
    (_, kind) => `<blockquote class="alert ${kind.toLowerCase()}"><p class="alert-label">${kind.toUpperCase()}</p><p>`,
  );
  const html = DOMPurify.sanitize(raw);
  const t = document.createElement('template');
  t.innerHTML = html;
  hit = { html, length: t.content.textContent.length };
  if (cache.size >= CACHE_CAP) cache.delete(cache.keys().next().value);
  cache.set(text, hit);
  return hit;
}

/// The first `n` visible characters of rendered HTML, structure kept, with
/// the caret (`live` or `parked`, or none) right after the last character
/// shown — inside whatever element it is in, so it rides a bold word or
/// the end of a list item rather than dropping to a line of its own. A code
/// chip is the exception: the caret follows it rather than sitting in it.
export function reveal(html, n, caret) {
  const t = document.createElement('template');
  t.innerHTML = html;
  let left = n;
  let last = null;
  const keep = (node) => {
    for (const child of Array.from(node.childNodes)) {
      if (left <= 0) {
        child.remove();
      } else if (child.nodeType === Node.TEXT_NODE) {
        if (child.data.length > left) child.data = child.data.slice(0, left);
        left -= child.data.length;
        // Whitespace between blocks counts (it is in `length`) but the
        // caret never parks on it: it would land after a closed paragraph.
        if (child.data.trim()) last = child;
      } else if (child.nodeType === Node.ELEMENT_NODE) {
        keep(child);
      }
    }
  };
  keep(t.content);
  if (caret) {
    const c = document.createElement('span');
    c.className = `cursor ${caret}`;
    const at = last?.parentElement?.closest('code') ?? last;
    if (at) at.after(c);
    else t.content.append(c);
  }
  return t.innerHTML;
}
