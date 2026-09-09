import { mount } from 'svelte';
import '../../app.css';
import { initPrefs } from '$lib/prefs';
import { installHelpBus } from '$lib/help-bus';
import SiteUsage from './SiteUsage.svelte';

installHelpBus('Site Usage');
initPrefs();
mount(SiteUsage, { target: document.body });
