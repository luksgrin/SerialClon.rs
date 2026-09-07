<script lang="ts">
  /** Window ▸ Standard Genetic Code: NCBI table 1, the layout of every textbook. */
  const AA = 'FFLLSSSSYY**CC*WLLLLPPPPHHQQRRRRIIIMTTTTNNKKSSRRVVVVAAAADDEEGGGG';
  const B = ['T', 'C', 'A', 'G'];
  const names: Record<string, string> = {
    A: 'Ala', R: 'Arg', N: 'Asn', D: 'Asp', C: 'Cys', E: 'Glu', Q: 'Gln', G: 'Gly', H: 'His', I: 'Ile', L: 'Leu', K: 'Lys',
    M: 'Met', F: 'Phe', P: 'Pro', S: 'Ser', T: 'Thr', W: 'Trp', Y: 'Tyr', V: 'Val', '*': 'Stop',
  };
  const aa = (i: number, j: number, k: number) => AA[i * 16 + j * 4 + k] ?? '';
</script>

<div class="card">
  <table>
    <thead><tr><th></th>{#each B as b (b)}<th>{b}</th>{/each}<th></th></tr></thead>
    <tbody>
      {#each B as first, i (first)}
        {#each B as third, k (third)}
          <tr>
            {#if k === 0}<th rowspan="4">{first}</th>{/if}
            {#each B as second, j (second)}
              {@const a = aa(i, j, k)}
              <td class:stop={a === '*'} class:start={first + second + third === 'ATG'}>
                <span class="codon">{first}{second}{third}</span> {names[a] ?? a}
              </td>
            {/each}
            <th>{third}</th>
          </tr>
        {/each}
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
    font-size: var(--fs-small);
  }
  th {
    color: var(--fg-muted);
    padding: 2px 4px;
  }
  td {
    padding: 2px 6px;
    border: 1px solid var(--border);
    white-space: nowrap;
  }
  .codon {
    font-family: var(--font-mono);
    margin-right: 4px;
  }
  .stop {
    color: var(--select-end);
    font-weight: 600;
  }
  .start {
    color: var(--select-begin);
    font-weight: 600;
  }
</style>
