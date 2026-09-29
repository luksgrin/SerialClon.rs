import { mount } from 'svelte';
import '../../app.css';
import { initPrefs } from '$lib/prefs';
import { installHelpBus } from '$lib/help-bus';
import { installEditMenu } from '$lib/edit-menu';
import Construct from './Construct.svelte';

installHelpBus('Build a Construct');
installEditMenu();
initPrefs();
mount(Construct, { target: document.body });
