#!/usr/bin/env bash
# Portable ownership tests and strict Android SDK compilation, not device proof.
set -euo pipefail
: "${JAVA_HOME:?JDK required}"
: "${ANDROID_JAR:?Installed platform android.jar required}"
BASE=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
ROOT=$(cd -- "$BASE/../../.." && pwd -P)
OUT=$(mktemp -d)
trap 'rm -rf -- "$OUT"' EXIT
mapfile -d '' SOURCES < <(find "$BASE/src" "$BASE/tests" "$ROOT/native/plugins/singles-platform/android/src/main/java" -name '*.java' -print0)
SOURCES+=("$ROOT/native/plugins/original-inference/android/src/com/nhlocal/singlesorter/inference/OriginalAssets.java" "$ROOT/native/plugins/original-inference/android/src/com/nhlocal/singlesorter/inference/OriginalInference.java")
"$JAVA_HOME/bin/javac" --release 8 -Xlint:all,-options -Werror -cp "$ANDROID_JAR" -d "$OUT" "${SOURCES[@]}"
"$JAVA_HOME/bin/java" -cp "$OUT" com.nhlocal.singlesorter.app.OwnedCompletionTest
