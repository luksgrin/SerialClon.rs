<script lang="ts">
  /** Window ▸ DNA Alphabet: the IUPAC degenerate nucleotide codes. */
  const rows: [string, string, string][] = [
    ['A', 'Adenine', 'A'], ['C', 'Cytosine', 'C'], ['G', 'Guanine', 'G'], ['T', 'Thymine', 'T'],
    ['R', 'puRine', 'A G'], ['Y', 'pYrimidine', 'C T'], ['S', 'Strong', 'C G'], ['W', 'Weak', 'A T'],
    ['K', 'Keto', 'G T'], ['M', 'aMino', 'A C'], ['B', 'not A', 'C G T'], ['D', 'not C', 'A G T'],
    ['H', 'not G', 'A C T'], ['V', 'not T', 'A C G'], ['N', 'aNy', 'A C G T'],
  ];
</script>

<div class="card">
  <table>
    <thead><tr><th>Code</th><th>Meaning</th><th>Bases</th></tr></thead>
    <tbody>
      {#each rows as [code, meaning, bases] (code)}
        <tr><td class="mono">{code}</td><td>{meaning}</td><td class="mono">{bases}</td></tr>
      {/each}
    </tbody>
  </table>
</div>

<style>
  .card {
    padding: 10px;
  }
  table {
    border-collapse: collapse;
    width: 100%;
  }
  th,
  td {
    padding: 3px 8px;
    text-align: left;
    border-bottom: 1px solid var(--border);
  }
  th {
    color: var(--fg-muted);
    font-size: var(--fs-small);
  }
  .mono {
    font-family: var(--font-mono);
  }
</style>
