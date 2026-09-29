import { mount } from 'svelte';
import '../../app.css';
import { initPrefs } from '$lib/prefs';
import { installHelpBus } from '$lib/help-bus';
import { installEditMenu } from '$lib/edit-menu';
import Adaptor from './Adaptor.svelte';

installHelpBus('Create an Adaptor');
installEditMenu();
initPrefs();
mount(Adaptor, { target: document.body });
