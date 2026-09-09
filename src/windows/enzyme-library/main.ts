import { mount } from 'svelte';
import '../../app.css';
import { initPrefs } from '$lib/prefs';
import { installHelpBus } from '$lib/help-bus';
import EnzymeLibrary from './EnzymeLibrary.svelte';

installHelpBus('Enzyme Library');
initPrefs();
mount(EnzymeLibrary, { target: document.body });
