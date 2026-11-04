# Generate the deterministic first-edition source list.
#
# Usage:
#   TYPELAKE_GROUP_SOURCE_OUTPUT=/absolute/path/groups-through-32.json \
#     gap -q -A --quitonbreak collections/finite-group-cohomology/sources/export-smallgroups.g

if LoadPackage("smallgrp") <> true then
  Error("SmallGrp is required");
fi;

if not IsBound(GAPInfo.SystemEnvironment.TYPELAKE_GROUP_SOURCE_OUTPUT) then
  Error("TYPELAKE_GROUP_SOURCE_OUTPUT is required");
fi;
output := GAPInfo.SystemEnvironment.TYPELAKE_GROUP_SOURCE_OUTPUT;
if output = "" then
  Error("TYPELAKE_GROUP_SOURCE_OUTPUT is required");
fi;

PrintTo(output,
  "{\n  \"contract\": \"finite-group-sources/smallgroups-v1\",\n",
  "  \"gap_version\": \"", GAPInfo.Version, "\",\n",
  "  \"smallgrp_version\": \"", PackageInfo("smallgrp")[1].Version, "\",\n",
  "  \"maximum_order\": 32,\n",
  "  \"groups\": [\n");

first := true;
for order in [1..32] do
  for index in [1..NumberSmallGroups(order)] do
    group := SmallGroup(order, index);
    elements := Elements(group);
    generators := Filtered(GeneratorsOfGroup(group), g -> g <> One(group));
    images := List(generators,
      generator -> List(elements, element -> Position(elements, element * generator) - 1));
    if not first then
      AppendTo(output, ",\n");
    fi;
    first := false;
    AppendTo(output,
      "    {\"lmfdb_label\": \"", order, ".", index,
      "\", \"small_groups_order\": ", order,
      ", \"small_groups_index\": ", index,
      ", \"permutation_degree\": ", order,
      ", \"generators\": ", images, "}");
  od;
od;

AppendTo(output, "\n  ]\n}\n");
QUIT;
