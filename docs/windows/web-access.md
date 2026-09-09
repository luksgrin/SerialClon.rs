# Web Access

Tools ▸ Web Access…. The original embedded a small web browser that scraped NCBI and EMBL pages;
those pages are long gone. SerialClon talks to the public APIs instead — and only when you press
a button. Nothing is fetched at startup except the release check described in
[Installation](../installation.md#updates).

## Fetching a record

Enter an **Accession** (NCBI nucleotide or protein: `J01749`, `NM_000546`, `P01857`; or an
EMBL/ENA accession), choose the **Source** (NCBI or ENA), press **Fetch**. The **Record** is shown
as GenBank / GenPept / EMBL text; **Create a New Sequence Window** parses it with the same reader
as File ▸ Open, so the Features come along with their coordinates.

Endpoints: NCBI E-utilities `efetch` (`rettype=gb` or `gp`) and the ENA browser API. Requests carry
a `tool` and `email` parameter as NCBI asks.

## BLAST

**Submit** sends the front-most sequence to NCBI BLAST (blastn for DNA, blastp for proteins —
also Sequence ▸ BLAST at the NCBI… and Protein ▸ BLAST Protein Sequence…). The request is
submitted (`CMD=Put`) and the result polled (`CMD=Get`) no more than once a minute per request
id, as NCBI's usage policy requires; the estimated time NCBI returns is shown. Results appear in
the **BLAST** panel. The two-sequence alignment of the Align window goes through the same
service (`BlastAlign.cgi`).

!!! note "Offline"
    Every network action fails with a plain message when there is no connection; nothing else in
    the application depends on the network.
