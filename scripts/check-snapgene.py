#!/usr/bin/env python3
"""Cross-check SerialClon.rs's SnapGene reader and writer against Biopython.

Run `cargo test -p sc-format --test snapgene_dump` first: it writes, for every
fixture under reference/sequences/snapgene/, what our reader saw
(`<name>.json`), the byte-exact round trip (`<name>.roundtrip.dna`) and a file
written from the model alone (`<name>.fresh.dna`) under target/tmp/snapgene-check/.

Then, in a venv with `pip install biopython`:

    python3 scripts/check-snapgene.py

For each fixture the original, the round trip and the fresh file are parsed
with Bio.SeqIO ("snapgene") and compared with our JSON: sequence, topology,
and the set of feature spans/strands/types. Biopython reads DNA files only,
so protein and RNA fixtures are reported as skipped.
"""
import json
import pathlib
import sys

from Bio import SeqIO

ROOT = pathlib.Path(__file__).resolve().parent.parent
FIXTURES = ROOT / "reference" / "sequences" / "snapgene"
CHECK = ROOT / "target" / "tmp" / "snapgene-check"


def bio_features(record):
    """Biopython features as a set of (type, lo, hi, strand, wraps)."""
    n = len(record.seq)
    out = set()
    for f in record.features:
        loc = f.location
        parts = getattr(loc, "parts", [loc])
        strand = f.location.strand or 1
        if len(parts) == 2 and int(parts[0].end) == n and int(parts[1].start) == 0:
            lo, hi, wraps = int(parts[1].end), int(parts[0].start) + 1, True
        elif len(parts) == 2 and int(parts[1].end) == n and int(parts[0].start) == 0:
            lo, hi, wraps = int(parts[0].end), int(parts[1].start) + 1, True
        else:
            lo, hi, wraps = int(loc.start) + 1, int(loc.end), False
        out.add((f.type, lo, hi, strand, wraps))
    return out


def our_features(report, include_primers=True):
    out = set()
    for f in report["features"]:
        if f["primer"] and not include_primers:
            continue
        out.add((f["kind"], f["lo"], f["hi"], f["strand"], f["wraps"]))
    return out


def compare(label, path, report, include_primers=True):
    try:
        rec = SeqIO.read(str(path), "snapgene")
    except Exception as e:  # noqa: BLE001
        return f"  {label}: SKIP (Biopython: {e})"
    problems = []
    if str(rec.seq) != report["sequence"]:
        problems.append("sequence differs")
    circular = rec.annotations.get("topology") == "circular"
    if circular != report["circular"]:
        problems.append(f"topology {circular} vs {report['circular']}")
    theirs = bio_features(rec)
    ours = our_features(report, include_primers)
    if theirs != ours:
        problems.append(f"features: only Biopython {sorted(theirs - ours)[:5]}, only ours {sorted(ours - theirs)[:5]}")
    return f"  {label}: {'OK' if not problems else 'MISMATCH ' + '; '.join(problems)}"


def main():
    if not CHECK.is_dir():
        sys.exit(f"{CHECK} missing: run `cargo test -p sc-format --test snapgene_dump` first")
    failures = 0
    checked = 0
    for js in sorted(CHECK.glob("*.json")):
        stem = js.stem
        report = json.loads(js.read_text())
        print(f"{stem} ({report['seq_type']}, {report['length']} bp, {len(report['features'])} features)")
        lines = [
            compare("original", FIXTURES / f"{stem}.dna", report),
            compare("roundtrip", CHECK / f"{stem}.roundtrip.dna", report),
            compare("fresh", CHECK / f"{stem}.fresh.dna", report, include_primers=False),
        ]
        for l in lines:
            print(l)
            if "MISMATCH" in l:
                failures += 1
            if "OK" in l:
                checked += 1
    print(f"\n{checked} comparisons OK, {failures} mismatches")
    sys.exit(1 if failures else 0)


if __name__ == "__main__":
    main()
