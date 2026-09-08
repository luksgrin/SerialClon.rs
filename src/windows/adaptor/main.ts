import { mount } from 'svelte';
import '../../app.css';
import { installHelpBus } from '$lib/help-bus';
import Adaptor from './Adaptor.svelte';

installHelpBus('Create an Adaptor');
mount(Adaptor, { target: document.body });
