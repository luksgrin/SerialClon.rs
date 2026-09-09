import { mount } from 'svelte';
import '../../app.css';
import { initPrefs } from '$lib/prefs';
import About from './About.svelte';

initPrefs();
mount(About, { target: document.body });
