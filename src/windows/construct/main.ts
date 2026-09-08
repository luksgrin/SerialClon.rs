import { mount } from 'svelte';
import '../../app.css';
import { installHelpBus } from '$lib/help-bus';
import Construct from './Construct.svelte';

installHelpBus('Build a Construct');
mount(Construct, { target: document.body });
