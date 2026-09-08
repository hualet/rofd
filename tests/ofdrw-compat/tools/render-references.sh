#!/usr/bin/env bash
# Regenerates tests/ofdrw-compat/references/ by rendering the migrated
# fixtures with ofdrw itself. Requires a JDK and Maven. One-time step; the
# generated PNGs and manifest.json are committed, so day-to-day test runs do
# not need Java.
#
# Usage: tests/ofdrw-compat/tools/render-references.sh [ofdrw checkout]
set -euo pipefail

OFD_RW_CHECKOUT="${1:-/home/hualet/projects/o/ofdrw}"
CRATE_DIR="$(cd "$(dirname "$0")/.." && pwd)"
WORK_DIR="$(mktemp -d)"
trap 'rm -rf "$WORK_DIR"' EXIT

echo ">> building ofdrw from $OFD_RW_CHECKOUT"
mvn -q -f "$OFD_RW_CHECKOUT/pom.xml" -pl ofdrw-converter -am -DskipTests install

echo ">> resolving classpath"
mvn -q -f "$OFD_RW_CHECKOUT/ofdrw-converter/pom.xml" \
    dependency:build-classpath -Dmdep.outputFile="$WORK_DIR/cp.txt"
CLASSPATH="$OFD_RW_CHECKOUT/ofdrw-converter/target/classes:$(cat "$WORK_DIR/cp.txt")"

echo ">> compiling RenderReferences.java"
javac -encoding UTF-8 -cp "$CLASSPATH" -d "$WORK_DIR" "$CRATE_DIR/tools/RenderReferences.java"

echo ">> rendering fixtures to $CRATE_DIR/references"
rm -rf "$CRATE_DIR/references"
java -Dfile.encoding=UTF-8 -cp "$WORK_DIR:$CLASSPATH" RenderReferences \
    "$CRATE_DIR/fixtures" "$CRATE_DIR/references"

echo ">> done; review the PNGs visually before committing"
