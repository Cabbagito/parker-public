import { mount } from 'svelte';
import App from './App.svelte';
import { inTauri } from './lib/shell.js';
import { applyTheme, savedTheme } from './lib/themes.js';
import './app.css';

// What differs by frame in CSS keys on this (app.css): the hidden cursor is
// for the tower's fullscreen surface, the web font for screens without one.
document.documentElement.dataset.shell = inTauri ? 'tauri' : 'web';

// Before anything mounts, so the first frame already wears it.
applyTheme(savedTheme(), { save: false });

export default mount(App, { target: document.getElementById('app') });
