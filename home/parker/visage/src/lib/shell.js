// Which frame the page hangs in, and what differs between them. One build
// serves both (docs/visage/porthole/README.md):
//
// - Tauri, on parker's own screen: `@tauri-apps/api`, verbatim.
// - A browser on the tailnet — giskard, the phone — reaching porthole
//   through `tailscale serve`: the same command names and the same payloads
//   over `fetch`, and the same event names off one EventSource.
//
// Nothing above engine.js can tell which frame it is in, and nothing above
// engine.js should ever need to. The one thing a frame adds is a capability,
// not a branch: `immersive` exists where it means something and is null
// where it does not, and callers ask `immersive?.toggle()`.

import { invoke as tauriInvoke } from '@tauri-apps/api/core';
import { listen as tauriListen } from '@tauri-apps/api/event';

export const inTauri = '__TAURI_INTERNALS__' in window;

const handlers = new Map(); // event name → Set of callbacks
let stream = null;

// The last face the server pushed. state.svelte.js polls `face_state` every
// 250 ms; on the tower that is a read of two tmpfs files, over a network it
// would be four requests a second from every page for a value that changes
// a few times a minute. So porthole watches the files on parker and pushes
// changes, and the poll is answered from here. When the stream drops this
// keeps the last face rather than falling back to polling a host that is
// not answering.
let face = null;

// The corner's spaces, pushed the same way and answered from here.
let spaces = null;

function fire(name, payload) {
  for (const cb of handlers.get(name) ?? []) cb({ payload });
}

function events() {
  if (stream) return;
  stream = new EventSource('/api/events');
  for (const name of ['engine-event', 'engine-status', 'face', 'spaces', 'levels']) {
    stream.addEventListener(name, (e) => {
      let payload;
      try {
        payload = JSON.parse(e.data);
      } catch {
        return;
      }
      if (name === 'face') face = payload;
      if (name === 'spaces') spaces = payload;
      fire(name, payload);
    });
  }
  // The stream dropping means parker is out of reach: powered off, off the
  // tailnet, or the server restarting. It is said as the engine being
  // unreachable because the status strip already has those words, and it is
  // true from here. EventSource retries on its own, and the server's first
  // event on reconnect is the real status, which clears the strip.
  stream.onerror = () => fire('engine-status', 'disconnected');
}

async function webInvoke(cmd, args = {}) {
  // `dismiss` flips a workspace on parker's screen. Off the tower there is
  // nothing to put away, so Esc's last tier does nothing here.
  if (cmd === 'dismiss') return null;
  if (cmd === 'face_state' && face) return face;
  if (cmd === 'spaces' && spaces) return spaces;
  const r = await fetch(`/api/${cmd}`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(args),
  });
  const text = await r.text();
  // Tauri's invoke rejects with the command's error string; so does this.
  if (!r.ok) throw text || `${r.status} ${r.statusText}`;
  return text ? JSON.parse(text) : null;
}

function webListen(name, cb) {
  events();
  if (!handlers.has(name)) handlers.set(name, new Set());
  handlers.get(name).add(cb);
  return Promise.resolve(() => handlers.get(name).delete(cb));
}

export const invoke = inTauri ? tauriInvoke : webInvoke;
export const listen = inTauri ? tauriListen : webListen;

// Immersive: the tower's screen, as near as a browser gets — Ctrl+F, or
// `/fullscreen` in the palette.
//
// Two browser features together. Real fullscreen (the Fullscreen API, not
// the browser's own full-screen mode) takes the tab strip and address bar
// away entirely. Keyboard Lock (Chrome and Edge) then hands the page the
// keys the browser keeps for itself: Ctrl+Tab stops switching tabs and
// switches views, Ctrl+N stops opening windows, and a plain Esc reaches
// visage's own Esc. Holding Esc leaves, and the browser says so on entry.
// Safari has the fullscreen and not the lock, so there it is fullscreen
// with the browser's keys still the browser's.
//
// Entering needs a user gesture — a browser rule, and a keypress counts.
// Shortcuts the OS keeps (Cmd+Tab; Ctrl+Space when macOS uses it to switch
// input sources) stay the OS's either way.
//
// Null on the tower, where a window manager already gave visage the whole
// screen, and in browsers with no Fullscreen API (the iPhone's).
async function toggleImmersive() {
  if (document.fullscreenElement) {
    navigator.keyboard?.unlock?.();
    await document.exitFullscreen();
    return;
  }
  await document.documentElement.requestFullscreen({ navigationUI: 'hide' });
  try {
    await navigator.keyboard?.lock?.();
  } catch {
    // No lock in this browser: still fullscreen, with its keys its own.
  }
}

export const immersive =
  !inTauri && document.fullscreenEnabled ? { toggle: toggleImmersive } : null;

if (immersive) {
  // Leaving by holding Esc ends fullscreen without passing through toggle;
  // the lock goes with it.
  document.addEventListener('fullscreenchange', () => {
    if (!document.fullscreenElement) navigator.keyboard?.unlock?.();
  });
}
