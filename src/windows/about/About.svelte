<script lang="ts">
  import { onMount } from 'svelte';
  import { appInfo, type AppInfo } from '$lib/tauri';
  let info = $state<AppInfo | null>(null);
  onMount(async () => {
    try {
      info = await appInfo();
    } catch {
      info = null;
    }
  });
</script>

<div class="about">
  <h1>SerialClon<span class="rs">.rs</span></h1>
  <p class="version">{info ? `version ${info.version}` : ''}</p>
  <p>An independent, open-source reimplementation of <b>Serial Cloner</b> (F. Perez, SerialBasics, 2004–2013): the same windows, interactions, keystrokes and file formats, on a stack that still builds.</p>
  <p class="small">Not affiliated with, endorsed by, or derived from the source code of Serial Cloner or SerialBasics.</p>
  <p class="small">Restriction enzyme data: REBASE {info?.rebase_release ?? ''} ({info?.enzyme_count ?? ''} enzymes), © Dr. Richard J. Roberts, New England Biolabs — free for academic use, rebase.neb.com.</p>
  <p class="small">"Gateway" is a trademark of Thermo Fisher Scientific. Built with Tauri {info?.tauri_version ?? ''}.</p>
  <p class="small">Licensed under MIT OR Apache-2.0, at your option. © 2026 SerialClon.rs contributors.</p>
</div>

<style>
  .about {
    padding: 16px 20px;
    -webkit-user-select: text;
    user-select: text;
  }
  h1 {
    margin: 0;
    font-size: 22px;
  }
  .rs {
    color: var(--accent);
  }
  .version {
    margin: 2px 0 12px;
    color: var(--fg-muted);
    font-family: var(--font-mono);
  }
  p {
    margin: 0 0 8px;
  }
  .small {
    font-size: var(--fs-small);
    color: var(--fg-muted);
  }
</style>
