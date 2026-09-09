import { mount } from 'svelte';
import '../../app.css';
import { initPrefs } from '$lib/prefs';
import { installHelpBus } from '$lib/help-bus';
import GraphicMap from './GraphicMap.svelte';

installHelpBus('Graphic Map');
initPrefs();
mount(GraphicMap, { target: document.body });
