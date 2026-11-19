#!/usr/bin/env python3
"""Independently check every abelian SmallGroups label and its invariants."""
import argparse
import json
from pathlib import Path
import subprocess


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--gap", default="gap")
    parser.add_argument("--source", type=Path, default=Path(__file__).resolve().parents[1] /
                        "collections/eilenberg-maclane/sources/finite-abelian-groups-through-32.json")
    parser.add_argument("--report", type=Path, required=True)
    options = parser.parse_args()
    source = json.loads(options.source.read_text())
    rows = [[row["order"], row["small_groups_index"], row["gap_abelian_invariants"]]
            for row in source["finite_groups"]]
    assert all(type(order) is int and type(index) is int and all(type(x) is int for x in invariants)
               for order, index, invariants in rows)
    script = f'''
if LoadPackage("smallgrp") <> true then Error("SmallGrp unavailable"); fi;
expected := {json.dumps(rows)};;
observed := [];;
for n in [1..{int(source["maximum_finite_order"])}] do
  for i in [1..NumberSmallGroups(n)] do
    g := SmallGroup(n,i);;
    if IsAbelian(g) then Add(observed, [n,i,AbelianInvariants(g)]); fi;
  od;
od;
if observed <> expected then Error("Abelian SmallGroups source differs"); fi;
Print("TYPELAKE_SOURCE|", GAPInfo.Version, "|", GAPInfo.PackagesInfo.smallgrp[1].Version, "|", Length(observed), "\\n");
QUIT_GAP(0);
'''
    result = subprocess.run([options.gap, "-q", "-A", "--quitonbreak"], input=script,
                            text=True, capture_output=True, timeout=120, check=True)
    line = next(line for line in result.stdout.splitlines() if line.startswith("TYPELAKE_SOURCE|"))
    _, gap_version, smallgrp_version, count = line.split("|")
    options.report.write_text(json.dumps(dict(status="passed", gap_version=gap_version,
                                            smallgrp_version=smallgrp_version,
                                            groups_checked=int(count),
                                            checks=["complete abelian index set", "exact primary invariants"]),
                                        indent=2) + "\n")


if __name__ == "__main__":
    main()
