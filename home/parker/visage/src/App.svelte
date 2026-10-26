<script>
  // One surface, two views, and the keymap that moves between them.
  //
  //   Ctrl+Tab          the other view — default ⇄ session
  //   Ctrl+K            the palette, open or closed — earlier threads with
  //                     their cards, search, and "/" for commands
  //                     (Palette.svelte; Ctrl+N / Ctrl+P or arrows move,
  //                     Ctrl+X closes the selected thread or opens it again)
  //   Ctrl+,            the controls, up or away — sound and each app's level,
  //                     what is playing, network, bluetooth, the look, quiet,
  //                     parker, the session, in vim's keys (Controls.svelte,
  //                     controls.svelte.js; docs/visage/controls/README.md)
  //   Ctrl+N            new chat, back in default (in the palette: next row; Ctrl+P previous)
  //   Ctrl+U / Ctrl+D   scroll half a screen; held, a steady glide
  //   Ctrl+Shift+K / J  scroll a line
  //   Ctrl+Shift+P / N  the previous / next of your turns
  //   PgUp / PgDn       scroll a screen
  //   Ctrl+G            the top of the thread; Ctrl+Shift+G the tail
  //   Ctrl+Space        dance, or stop dancing (Ctrl+B too — b for beat)
  //   Ctrl+F            immersive: real fullscreen, browser keys handed over
  //                     (porthole only; shell.js)
  //   Enter / Shift+Enter   send / newline
  //   Esc               close the palette · stop the dance · stop the stream ·
  //                     clear the box · put visage away. Never ^, — esc is
  //                     its own there (clear a find, leave a mode, undo)
  //   typing anywhere lands in the input, mid-keystroke — and so do
  //   Backspace, arrows, Enter and paste; a click puts the caret back
  //
  // Two views is a toggle, not an axis, and a toggle wants one key: Ctrl+Tab
  // means "the other one", which is the whole of what there is to say. It
  // replaced Ctrl+± when `deep` was cut — a directional pair for a binary
  // state makes half of every press a no-op. Nothing else in the app moves
  // the view: layout changes because you asked, never because the content
  // grew.
  //
  // The chrome — corner, WEARS, the view indicator — is identical in every
  // view: same position, same size, same style. Only the face and the
  // transcript region move.
  //
  // Esc is the only key that does not touch layout. It cancels whatever is
  // in front of you, in whatever order things are cancellable — which makes
  // it the one key you can press without first working out where you are.
  import { onMount } from 'svelte';
  import { fade } from 'svelte/transition';
  import Stage from './lib/Stage.svelte';
  import Chat from './lib/Chat.svelte';
  import Corner from './lib/Corner.svelte';
  import Wears from './lib/Wears.svelte';
  import Brain from './lib/Brain.svelte';
  import ViewIndicator from './lib/ViewIndicator.svelte';
  import Palette from './lib/Palette.svelte';
  import Controls from './lib/Controls.svelte';
  import KeyLog from './lib/KeyLog.svelte';
  import { other } from './lib/views.js';
  import {
    app,
    expression,
    init,
    newChat,
    interrupt,
    dismiss,
    openPalette,
    closePalette,
    resume,
    notice,
    pickBrain,
    pickTheme,
    heardLevels,
  } from './lib/state.svelte.js';
  import { stir, isDancing } from './lib/motion.svelte.js';
  import {
    ctl,
    key as controlsKey,
    openControls,
    closeControls,
    refresh as readControls,
    runSlash,
    heard as controlsHeard,
  } from './lib/controls.svelte.js';
  import { engine } from './lib/engine.js';
  import { immersive } from './lib/shell.js';

  let chat = $state();
  let palette = $state();
  let controls = $state();
  let expr = $state('idle');

  // Whether parker is mid-turn: a prompt from here still in flight, or the
  // newest turn not finished (one started from parker-chat, say). The face
  // keeps its idle flair to itself while this is true.
  const working = $derived(app.busy || app.messages.at(-1)?.done === false);

  // Panic-tapping Esc to stop parker must not sail past the stop and throw
  // the window away. One interrupt buys a moment of deafness.
  const GRACE_MS = 500;
  let interruptedAt = 0;

  onMount(() => {
    init();
    // A level the keys on the desk moved (door's `watch_levels`): a quiet
    // line in the transcript, and ^,'s row at once if it is up.
    engine.onLevels((v) => {
      heardLevels(v);
      controlsHeard(v);
    });
    // The worn face ticks on its own clock, not on renders.
    const t = setInterval(() => (expr = expression()), 250);
    requestAnimationFrame(() => chat?.focusInput());
    return () => clearInterval(t);
  });

  // The caret belongs in the box. On the tower nothing takes it away — the
  // pointer is hidden and nobody clicks — but in a browser a click anywhere
  // moves focus to the page, and from then on only letters found their way
  // back (the typing-anywhere rule below). Everything else went nowhere,
  // which is what "the input came loose" looked like from giskard.
  //
  // So: a click that did not select anything gives the caret back, and so
  // does the window regaining focus. A click that *did* select is someone
  // copying from the transcript, and is left alone. While the palette is up
  // the caret's place is its box, not the chat's.
  const overlay = () => app.palette || app.controls;
  const typing = () =>
    document.activeElement?.tagName === 'TEXTAREA' || document.activeElement?.tagName === 'INPUT';

  /// `?keys` on the porthole's address: what the page hears (KeyLog.svelte).
  const keylog = new URLSearchParams(location.search).has('keys');

  function reclaim() {
    if (typing()) return;
    if (!window.getSelection()?.isCollapsed) return;
    if (app.controls) controls?.focus();
    else if (app.palette) palette?.focus();
    else chat?.focusInput();
  }

  function go(view) {
    if (view === app.view) return;
    app.view = view;
    requestAnimationFrame(() => chat?.focusInput());
  }

  /// Ctrl+N and the palette's `new`. A new chat starts on the home screen:
  /// the session view is for a conversation, and there is none yet. Ctrl+Tab
  /// still takes it to session from there if that is where you want it.
  function fresh() {
    newChat();
    go('default');
    chat?.focusInput();
  }

  /// Dance is a *mode*, not a view, so it goes through the same verb the
  /// keybind and the couch and parker himself all use — `parker reach dance`.
  /// A second implementation living in the app would be one that drifts,
  /// which is the same argument `dismiss` already makes about `parker visage`.
  function toggleDance() {
    const routine = isDancing() ? 'stop' : 'auto';
    // Zero hold: a dance you started by hand ends when you say so, not on a
    // timer. The default minute is for parker deciding to dance on his own.
    engine.dance(routine, isDancing() ? undefined : 0).catch(() => {});
  }

  /// Ctrl+K, the one key for the one overlay: up if it is down, down if it
  /// is up, whatever has focus. The caret goes to the palette's box while it
  /// is up and back to the chat's when it is not.
  function togglePalette() {
    if (app.palette) shutPalette();
    else {
      if (app.controls) closeControls();
      document.activeElement?.blur();
      // The palette's "/" for the machine is made from ^,'s list: read it.
      readControls();
      openPalette();
    }
  }

  /// Ctrl+, — the controls, in the transcript's place, parker on the rail.
  /// One overlay at a time: the palette goes down for it.
  function toggleControls() {
    if (app.controls) {
      closeControls();
      chat?.focusInput();
      return;
    }
    if (app.palette) closePalette();
    document.activeElement?.blur();
    openControls();
  }

  function shutPalette() {
    closePalette();
    chat?.focusInput();
  }

  /// What a palette pick does. A thread is resumed (and given the session
  /// view); a command goes through the same path its key does, so a command
  /// and its key can never disagree.
  async function pickPalette(row) {
    if (row.kind === 'thread') {
      await resume(row.id);
      if (app.view === 'default') go('session');
      else chat?.focusInput();
      return;
    }
    // A look goes on before the palette comes down, so coming down has no
    // preview to take back off.
    if (row.id === 'visage') pickTheme(row.arg);
    shutPalette();
    // The machine's "/" commands are ^,'s rows (commands.js): run as ^,
    // runs them, through the same door call.
    if (row.machine) {
      runSlash(row.id, row.arg);
      return;
    }
    switch (row.id) {
      case 'controls':
        toggleControls();
        break;
      case 'new':
        fresh();
        break;
      case 'view':
        go(other(app.view));
        break;
      case 'dance':
        // Only when the dance is not already what was asked for.
        if (isDancing() !== (row.arg === 'auto')) toggleDance();
        break;
      case 'fullscreen':
        // A palette pick is a keypress, so it counts as the gesture the
        // browser wants before it goes fullscreen.
        immersive?.toggle().catch(() => {});
        break;
      case 'theme': {
        const said = `theme ${row.arg}`;
        // Held until it answers: `set` is a full rebuild and takes a while.
        notice(`${said} …`, 0);
        engine
          .theme(row.arg.split(' '))
          .then((line) => notice(line ? `${said}: ${line}` : `${said}: done`))
          .catch((e) => notice(`${said} failed: ${e}`, 12000));
        break;
      }
      case 'brain':
        pickBrain(row.arg);
        break;
    }
  }

  async function onEscape() {
    if (Date.now() - interruptedAt < GRACE_MS) return;
    // The palette is the frontmost cancellable thing while it is up. Its box
    // catches Esc itself; this is for when the box has lost focus.
    if (app.palette) {
      shutPalette();
      return;
    }
    // Dance is the frontmost cancellable thing, so it tiers in above the
    // stream. No new rule: Esc already means "cancel whatever is in front of
    // you, in whatever order things are cancellable", and a dance is very
    // much in front of you.
    if (isDancing()) {
      toggleDance();
      return;
    }
    if (await interrupt()) {
      interruptedAt = Date.now();
      return;
    }
    if (chat?.clearInput()) return;
    dismiss();
  }

  function onKey(e) {
    // Any key at all is something happening to parker: it restarts the
    // settling period, and if he had drifted off it wakes him with a start.
    stir();

    // A key the text system is still composing with (an accent, a dead key,
    // an input method's candidate) is the text system's, not ours: Esc
    // cancels the composition, Enter commits it. Acting on those too would
    // close things, or send a half-written character. 229 is how browsers
    // spell "composing" on keydown where isComposing is not set.
    if (e.isComposing || e.keyCode === 229) return;

    // ^, from anywhere: up, or away.
    if (e.ctrlKey && !e.altKey && !e.metaKey && e.key === ',') {
      e.preventDefault();
      toggleControls();
      return;
    }

    // ^, up: its keys first (controls.svelte.js), then the few of App's
    // that still mean something over it. Nothing reaches the conversation
    // underneath — not a letter, not ^N, not a paste.
    if (app.controls) {
      if (controlsKey(e)) return;
      if (e.ctrlKey && !e.altKey && !e.metaKey) {
        const k = e.key.toLowerCase();
        if (k === 'k') {
          e.preventDefault();
          togglePalette();
        } else if (e.key === ' ' || e.code === 'Space' || k === 'b') {
          e.preventDefault();
          toggleDance();
        } else if (k === 'f' && immersive) {
          e.preventDefault();
          immersive.toggle().catch(() => {});
        } else if (k !== 'v' && k !== 'c' && k !== 'x' && k !== 'a') {
          e.preventDefault();
        }
        return;
      }
      // FIND and ASK type into ^,'s own line; a letter that arrives while it
      // has lost focus (a click elsewhere) still lands in it.
      if ((ctl.mode === 'find' || ctl.mode === 'ask') && !typing() && e.key.length === 1) {
        controls?.focus();
        return;
      }
      if (e.key === 'Tab') e.preventDefault();
      return;
    }

    // The palette up and its box not focused — a click elsewhere, on the
    // porthole. Its keys still reach it, the same function handling them
    // as when the box has focus, and typing still lands in its query.
    if (app.palette && !typing()) {
      if (e.key.length === 1 && !e.ctrlKey && !e.metaKey && !e.altKey) {
        e.preventDefault();
        palette?.type(e.key);
        return;
      }
      if (palette?.key(e)) return;
    }

    if (e.key === 'Escape') {
      e.preventDefault();
      onEscape();
      return;
    }

    // Paste lands in a box even when the box had lost focus: focus moves
    // now, during the keydown, and the paste that follows it goes to
    // whatever has focus by then. Cmd+V on a Mac, Ctrl+V elsewhere.
    if ((e.metaKey || e.ctrlKey) && e.key === 'v' && !typing()) {
      if (overlay()) palette?.focus();
      else chat?.focusInput();
      return;
    }

    // A screen at a time, the terminal's scrollback keys. The box is a few
    // lines tall and has no use for them; the lists own them while up.
    if (
      (e.key === 'PageUp' || e.key === 'PageDown') &&
      !overlay() &&
      !e.ctrlKey &&
      !e.metaKey &&
      !e.altKey
    ) {
      e.preventDefault();
      chat?.scroll('page', e.key === 'PageDown' ? 1 : -1);
      return;
    }

    if (e.ctrlKey && !e.altKey && !e.metaKey) {
      // The transcript (SCROLLS). Ahead of every letter below, because
      // Ctrl+Shift+K is a line and not the palette, and matched lowercased
      // so Caps Lock changes nothing.
      const id = (e.shiftKey ? '⇧' : '') + e.key.toLowerCase();
      const motion = SCROLLS[id];
      if (motion) {
        e.preventDefault();
        // Held, the half-page jump becomes a steady glide until let go.
        if (motion[0] === 'half' && held(id, e)) chat?.cruise(motion[1]);
        else chat?.scroll(...motion);
        return;
      }
      // The palette. K because every palette anyone has ever used is on K.
      // Earlier threads live in it too: ^Y was a second overlay for them,
      // and two overlays stacked and toggled past each other (Oct 2026).
      if (e.key === 'k') {
        e.preventDefault();
        togglePalette();
        return;
      }
      // A mode rather than a view, so it deliberately is not a view key.
      // Space because nothing else uses it and because
      // space-is-playback is the one music idiom everybody already has.
      // Ctrl+B is the same toggle for screens where Ctrl+Space is not ours
      // to have — macOS takes it to switch input sources when there is more
      // than one, before any page hears it. b for beat.
      if (e.key === ' ' || e.code === 'Space' || e.key === 'b') {
        e.preventDefault();
        toggleDance();
        return;
      }
      // Immersive (shell.js), where it exists. On the tower there is no
      // such thing, and Ctrl+F falls through as it always did.
      if (e.key === 'f' && immersive) {
        e.preventDefault();
        immersive.toggle().catch(() => {});
        return;
      }
      // The other view. preventDefault is not optional: Tab's default is
      // focus traversal, and a view change that also moved the caret out of
      // the input would cost you the next thing you typed.
      if (e.key === 'Tab') {
        e.preventDefault();
        go(other(app.view));
        return;
      }
      // Swallowed, not bound. Ctrl+± is WebKit page zoom, and visage has no
      // way back from a zoomed webview — the font size is set once for one
      // screen and there is nothing here worth changing it with.
      if (e.key === '=' || e.key === '+' || e.key === '-' || e.key === '_' || e.key === '0') {
        e.preventDefault();
        return;
      }
      if (e.key === 'n') {
        e.preventDefault();
        fresh();
        return;
      }
      return;
    }

    // Typing anywhere means talking: if the input lost focus somehow, the
    // character you meant to type is still the first one in it.
    if (
      !app.palette &&
      e.key.length === 1 &&
      !e.ctrlKey &&
      !e.metaKey &&
      !e.altKey &&
      document.activeElement?.tagName !== 'TEXTAREA' &&
      document.activeElement?.tagName !== 'INPUT'
    ) {
      const typed = e.key;
      e.preventDefault();
      chat?.seed(typed, true);
      return;
    }

    // The rest of typing — keys that edit or move rather than insert — find
    // the box the same way. Enter sends what is there; the others take
    // focus first and then do their usual thing to the box, caret where it
    // was left.
    if (!overlay() && !typing() && !e.ctrlKey && !e.metaKey && !e.altKey) {
      if (e.key === 'Enter' && !e.shiftKey) {
        e.preventDefault();
        chat?.focusInput();
        chat?.submit();
        return;
      }
      if (EDITING.has(e.key)) chat?.focusInput();
    }
  }

  /// Ctrl+key -> a transcript motion (Chat.svelte `scroll`), `⇧` for Shift.
  /// vim's grains under the one modifier visage already means: d/u half a
  /// screen and Shift for the finer j/k line, since Ctrl+K is the palette.
  /// g is the top and Shift+G the tail, as gg and G are. Shift+N/P steps
  /// through your turns — next and previous, as N and P move every list
  /// here. Not Shift+U: GTK's input method takes Ctrl+Shift+U for typing
  /// a code point, before the page ever hears it.
  const SCROLLS = {
    d: ['half', 1],
    u: ['half', -1],
    '⇧j': ['line', 1],
    '⇧k': ['line', -1],
    '⇧n': ['turn', 1],
    '⇧p': ['turn', -1],
    g: ['end', -1],
    '⇧g': ['end', 1],
  };

  /// Whether a keydown is a held key's repeat. Chrome says so in `repeat`;
  /// the timing says so everywhere, WebKitGTK included, which has not been
  /// checked to set it. Repeats come 29 ms apart here (hyprland.nix) and no
  /// hand double-taps inside 100.
  let lastMotion = '';
  let lastMotionAt = 0;
  function held(id, e) {
    const soon = id === lastMotion && e.timeStamp - lastMotionAt < 100;
    lastMotion = id;
    lastMotionAt = e.timeStamp;
    return e.repeat || soon;
  }

  /// Letting go of Ctrl+D/U, or of Ctrl, stops the glide where it is.
  function onKeyUp(e) {
    if (e.key === 'Control' || /^[du]$/i.test(e.key)) chat?.halt();
  }

  const EDITING = new Set([
    'Backspace',
    'Delete',
    'ArrowLeft',
    'ArrowRight',
    'ArrowUp',
    'ArrowDown',
    'Home',
    'End',
  ]);
</script>

<svelte:window onkeydown={onKey} onkeyup={onKeyUp} onmouseup={reclaim} onfocus={reclaim} />

<main>
  <!-- Where the title bar was. Installed as an app with its title bar
       hidden (window controls overlay, public/manifest.webmanifest), visage
       fills the window edge to edge, and this strip is the part you drag
       it by. Its size comes from the browser's titlebar-area variables,
       which exist only in that mode, so anywhere else it is zero by zero. -->
  <div class="titlebar" aria-hidden="true"></div>
  <!-- ^, takes the session shape: parker on the rail, the controls where the
       transcript was. The conversation is only hidden, never unmounted, so
       its scroll and its half-typed line are there when ^, goes away. -->
  <Stage view={app.controls ? 'session' : app.view} expression={expr} travel={app.travel} {working} />

  <div class="under" class:away={app.controls}>
    <Chat bind:this={chat} view={app.view} />
  </div>

  {#if app.controls}
    <Controls bind:this={controls} />
  {/if}

  <Corner />
  <Wears worn={expr} />
  <Brain brain={app.brain?.name} />
  <ViewIndicator view={app.controls ? 'session' : app.view} />

  {#if keylog}
    <KeyLog />
  {/if}

  {#if app.palette}
    <Palette bind:this={palette} onpick={pickPalette} onclose={shutPalette} />
  {/if}

  {#if app.engineStatus === 'disconnected'}
    <div class="status">engine unreachable — retrying</div>
  {/if}

  {#if app.notice}
    <div class="notice" transition:fade={{ duration: 200 }}>{app.notice}</div>
  {/if}
</main>

<style>
  main {
    position: relative;
    height: 100vh;
    background: var(--bg);
    color: var(--fg);
    overflow: hidden;
  }
  .under.away {
    visibility: hidden;
  }
  .titlebar {
    position: fixed;
    top: env(titlebar-area-y, 0);
    left: env(titlebar-area-x, 0);
    width: env(titlebar-area-width, 0);
    height: env(titlebar-area-height, 0);
    z-index: 10;
    app-region: drag;
    -webkit-app-region: drag;
  }
  .status {
    position: absolute;
    bottom: 0.75rem;
    right: 1rem;
    color: var(--bad);
    font-size: 0.8rem;
    z-index: 3;
  }

  /* A passing line (notice()): the palette's /theme result. Quiet, and in
     the corner opposite the view indicator. */
  .notice {
    position: absolute;
    bottom: 30px;
    right: 48px;
    z-index: 3;
    font-size: 20px;
    letter-spacing: 0.04em;
    color: var(--fg-45);
  }

</style>
