import { mount } from 'svelte';
import '../../app.css';
import { installHelpBus } from '$lib/help-bus';
import GraphicMap from './GraphicMap.svelte';

installHelpBus('Graphic Map');
mount(GraphicMap, { target: document.body });
