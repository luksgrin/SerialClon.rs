import { mount } from 'svelte';
import '../../app.css';
import { initPrefs } from '$lib/prefs';
import { installHelpBus } from '$lib/help-bus';
import CodonUsage from './CodonUsage.svelte';

installHelpBus('Codon Usage');
initPrefs();
mount(CodonUsage, { target: document.body });
