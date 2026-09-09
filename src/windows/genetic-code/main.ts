import { mount } from 'svelte';
import '../../app.css';
import { initPrefs } from '$lib/prefs';
import GeneticCode from './GeneticCode.svelte';

initPrefs();
mount(GeneticCode, { target: document.body });
