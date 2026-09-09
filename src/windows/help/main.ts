import { mount } from 'svelte';
import '../../app.css';
import { initPrefs } from '$lib/prefs';
import Help from './Help.svelte';

initPrefs();
mount(Help, { target: document.body });
