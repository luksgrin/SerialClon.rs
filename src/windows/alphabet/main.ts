import { mount } from 'svelte';
import '../../app.css';
import { initPrefs } from '$lib/prefs';
import Alphabet from './Alphabet.svelte';

initPrefs();
mount(Alphabet, { target: document.body });
