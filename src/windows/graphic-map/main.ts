import { mount } from 'svelte';
import '../../app.css';
import { initPrefs } from '$lib/prefs';
import { installHelpBus } from '$lib/help-bus';
import { installEditMenu } from '$lib/edit-menu';
import GraphicMap from './GraphicMap.svelte';

installHelpBus('Graphic Map');
installEditMenu();
initPrefs();
mount(GraphicMap, { target: document.body });
