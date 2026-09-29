import { mount } from 'svelte';
import '../../app.css';
import { initPrefs } from '$lib/prefs';
import { installHelpBus } from '$lib/help-bus';
import { installEditMenu } from '$lib/edit-menu';
import SiteUsage from './SiteUsage.svelte';

installHelpBus('Site Usage');
installEditMenu();
initPrefs();
mount(SiteUsage, { target: document.body });
