import { mount } from 'svelte';
import '../../app.css';
import { initPrefs } from '$lib/prefs';
import { installHelpBus } from '$lib/help-bus';
import Construct from './Construct.svelte';

installHelpBus('Build a Construct');
initPrefs();
mount(Construct, { target: document.body });
