# Generate the reader-facing GAP structure descriptions used by TypeBase.
#
# These names are external catalogue metadata, not mathematical identity.  They
# therefore live beside, rather than inside, the immutable permutation source.
#
# Usage:
#   TYPEBASE_GROUP_NAMES_OUTPUT=/absolute/path/standard-names-through-32.json \
#     gap -q -A --quitonbreak \
#       collections/finite-group-cohomology/sources/export-standard-names.g

if LoadPackage("smallgrp") <> true then
  Error("SmallGrp is required");
fi;
SizeScreen([4096, 4096]);

if not IsBound(GAPInfo.SystemEnvironment.TYPEBASE_GROUP_NAMES_OUTPUT) then
  Error("TYPEBASE_GROUP_NAMES_OUTPUT is required");
fi;
output := GAPInfo.SystemEnvironment.TYPEBASE_GROUP_NAMES_OUTPUT;
if output = "" then
  Error("TYPEBASE_GROUP_NAMES_OUTPUT is required");
fi;

PrintTo(output,
  "{\n  \"contract\": \"finite-group-standard-names/gap-structure-description-v1\",\n",
  "  \"gap_version\": \"", GAPInfo.Version, "\",\n",
  "  \"smallgrp_version\": \"", PackageInfo("smallgrp")[1].Version, "\",\n",
  "  \"maximum_order\": 32,\n",
  "  \"names\": [\n");

first := true;
for order in [1..32] do
  for index in [1..NumberSmallGroups(order)] do
    if not first then
      AppendTo(output, ",\n");
    fi;
    first := false;
    AppendTo(output,
      "    {\"lmfdb_label\": \"", order, ".", index,
      "\", \"standard_name\": \"",
      StructureDescription(SmallGroup(order, index)), "\"}");
  od;
od;

AppendTo(output, "\n  ]\n}\n");
QUIT;
