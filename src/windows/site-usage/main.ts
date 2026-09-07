import { mount } from 'svelte';
import '../../app.css';
import { installHelpBus } from '$lib/help-bus';
import SiteUsage from './SiteUsage.svelte';

installHelpBus('Site Usage');
mount(SiteUsage, { target: document.body });
