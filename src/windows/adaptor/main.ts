import { mount } from 'svelte';
import '../../app.css';
import { initPrefs } from '$lib/prefs';
import { installHelpBus } from '$lib/help-bus';
import Adaptor from './Adaptor.svelte';

installHelpBus('Create an Adaptor');
initPrefs();
mount(Adaptor, { target: document.body });
