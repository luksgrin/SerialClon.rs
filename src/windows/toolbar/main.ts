import { mount } from 'svelte';
import '../../app.css';
import { installHelpBus } from '$lib/help-bus';
import Toolbar from './Toolbar.svelte';

installHelpBus('Toolbar');
mount(Toolbar, { target: document.body });
