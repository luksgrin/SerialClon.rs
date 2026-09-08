import { mount } from 'svelte';
import '../../app.css';
import { installHelpBus } from '$lib/help-bus';
import Web from './Web.svelte';

installHelpBus('Web Access');
mount(Web, { target: document.body });
