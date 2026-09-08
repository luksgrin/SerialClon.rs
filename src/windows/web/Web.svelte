<script lang="ts">
  /**
   * Web Access (Tools menu; changelog 1.3, 2.0, 2.1): fetch an NCBI or EMBL
   * record by accession and open it as a Sequence Window, or submit a BLAST
   * search of an open sequence. Everything here is an explicit user action;
   * nothing is fetched on its own.
   */
  import { onMount } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import { message } from '@tauri-apps/plugin-dialog';
  import { api, type RecordView } from '$lib/doc';

  let tab = $state<'fetch' | 'blast'>('fetch');
  let source = $state<'ncbi' | 'ncbi-protein' | 'ena'>('ncbi');
  let accession = $state('');
  let record = $state<RecordView | null>(null);
  let busy = $state(false);

  let docs = $state<[number, string][]>([]);
  let queryId = $state<number | null>(null);
  let program = $state('blastn');
  let database = $state('nt');
  let job = $state<{ rid: string; status: string; wait: number; text: string } | null>(null);
  let pollTimer: ReturnType<typeof setTimeout> | undefined;

  async function fetchRecord() {
    busy = true;
    try {
      record = await api.webFetch(source, accession);
    } catch (e) {
      record = null;
      await message(String(e), { title: 'Web Access', kind: 'error' });
    } finally {
      busy = false;
    }
  }
  async function createWindow() {
    if (!record) return;
    try {
      await api.webCreateDocument(record.text, record.name, record.format === 'genpept');
    } catch (e) {
      await message(String(e), { title: 'Create a New Sequence Window', kind: 'error' });
    }
  }

  async function submit() {
    if (queryId === null) return;
    busy = true;
    try {
      const j = await api.blastSubmit(queryId, 0, 0, program, database, null);
      job = { rid: j.rid, status: 'submitted', wait: Math.max(5, j.rtoe), text: '' };
      schedule(Math.max(5, j.rtoe));
    } catch (e) {
      await message(String(e), { title: 'BLAST', kind: 'error' });
    } finally {
      busy = false;
    }
  }
  function schedule(seconds: number) {
    clearTimeout(pollTimer);
    pollTimer = setTimeout(poll, seconds * 1000);
  }
  async function poll() {
    if (!job) return;
    try {
      const s = await api.blastStatus(job.rid);
      job = { ...job, status: s.status, wait: s.wait };
      if (s.status === 'ready') job = { ...job, text: await api.blastResult(job.rid) };
      else if (s.status === 'waiting') schedule(Math.max(s.wait, 15));
    } catch (e) {
      job = { ...job, status: `error: ${e}` };
    }
  }

  onMount(() => {
    void api.listDocuments().then((d) => {
      docs = d;
      if (queryId === null && d[0]) queryId = d[0][0];
    });
    const t = setInterval(() => void api.listDocuments().then((d) => (docs = d)), 3000);
    let unlisten: (() => void) | undefined;
    void listen<{ id: number; from: number; to: number; program: string }>('web:preset', (e) => {
      tab = 'blast';
      queryId = e.payload.id;
      program = e.payload.program;
      database = e.payload.program === 'blastp' ? 'nr' : 'nt';
    }).then((u) => (unlisten = u));
    return () => {
      clearInterval(t);
      clearTimeout(pollTimer);
      unlisten?.();
    };
  });
</script>

<div class="web">
  <div class="tabs" role="tablist">
    <button type="button" role="tab" aria-selected={tab === 'fetch'} onclick={() => (tab = 'fetch')} data-help-name="Fetch" data-help="Fetch a sequence record from the NCBI (nucleotide or protein) or from EMBL/ENA by accession number.">Fetch a record</button>
    <button type="button" role="tab" aria-selected={tab === 'blast'} onclick={() => (tab = 'blast')} data-help-name="BLAST" data-help="Submit an open sequence to the NCBI BLAST server.">BLAST</button>
  </div>

  {#if tab === 'fetch'}
    <div class="row">
      <select bind:value={source} data-help-name="Source" data-help="NCBI nucleotide (GenBank), NCBI protein (GenPept) or EMBL/ENA.">
        <option value="ncbi">NCBI nucleotide</option>
        <option value="ncbi-protein">NCBI protein</option>
        <option value="ena">EMBL / ENA</option>
      </select>
      <input type="text" placeholder="accession, e.g. J01749" bind:value={accession} onkeydown={(e) => { if (e.key === 'Enter') void fetchRecord(); }} spellcheck="false" data-help-name="Accession" data-help="The accession number (or GI) of the record to fetch." />
      <button type="button" onclick={fetchRecord} disabled={busy || !accession.trim()}>Fetch</button>
      <button type="button" class="primary" onclick={createWindow} disabled={!record} data-help-name="Create a New Sequence Window" data-help="Parses the fetched record (sequence, comments and Features) into a new Sequence Window.">Create a New Sequence Window…</button>
    </div>
    <pre class="record" data-help-name="Record" data-help="The fetched record as served (GenBank, GenPept or EMBL flat file).">{record?.text ?? (busy ? 'Fetching…' : 'Enter an accession and press Fetch. SerialClon only goes online when you ask it to.')}</pre>
  {:else}
    <div class="row">
      <label>Query <select bind:value={queryId}>{#each docs as [id, name] (id)}<option value={id}>{name}</option>{/each}</select></label>
      <label>Program <select bind:value={program}><option value="blastn">blastn</option><option value="megablast">megablast</option><option value="blastp">blastp</option><option value="blastx">blastx</option><option value="tblastn">tblastn</option></select></label>
      <label>Database <input type="text" bind:value={database} class="db" /></label>
      <button type="button" class="primary" onclick={submit} disabled={busy || queryId === null} data-help-name="Submit" data-help="Submits the whole sequence of the chosen window to the NCBI BLAST server; the answer is polled at the rate the NCBI asks for (once a minute).">Submit to NCBI</button>
    </div>
    {#if job}
      <div class="hint">Request {job.rid}: {job.status}{job.status === 'waiting' ? ` — next check in ${Math.max(job.wait, 15)} s` : ''}</div>
    {/if}
    <pre class="record">{job?.text ?? ''}</pre>
  {/if}
</div>

<style>
  .web {
    display: flex;
    flex-direction: column;
    height: 100vh;
    box-sizing: border-box;
    padding: 8px;
    gap: 6px;
  }
  .tabs {
    display: flex;
    gap: 2px;
    border-bottom: 1px solid var(--border);
  }
  .tabs button {
    padding: 3px 14px;
    border: 1px solid transparent;
    border-bottom: none;
    border-radius: var(--radius) var(--radius) 0 0;
    background: transparent;
  }
  .tabs button[aria-selected='true'] {
    border-color: var(--border);
    background: var(--bg-raised);
    margin-bottom: -1px;
  }
  .row {
    display: flex;
    gap: 8px;
    align-items: center;
    flex-wrap: wrap;
  }
  .row label {
    display: flex;
    gap: 4px;
    align-items: center;
  }
  input,
  select {
    font: inherit;
  }
  input[type='text'] {
    min-width: 200px;
  }
  .db {
    min-width: 80px;
  }
  .primary {
    background: var(--accent);
    color: #fff;
  }
  .record {
    flex: 1;
    margin: 0;
    padding: 8px;
    overflow: auto;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    background: #fff;
    color: #111;
    font-family: ui-monospace, Menlo, Consolas, monospace;
    font-size: 11px;
    -webkit-user-select: text;
    user-select: text;
  }
  .hint {
    color: var(--fg-muted);
    font-size: var(--fs-small);
  }
</style>
