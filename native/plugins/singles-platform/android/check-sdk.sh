#!/usr/bin/env bash
set -euo pipefail
: "${ANDROID_HOME:?Set ANDROID_HOME to the existing Android SDK}"
: "${JAVA_HOME:?Set JAVA_HOME to JDK 17 or later}"
base="$(cd "$(dirname "$0")" && pwd)"
out="$(mktemp -d)"
trap 'rm -rf "$out"' EXIT
mapfile -t sources < <(find "$base/src/main/java" "$base/src/test/java" -name '*.java' -print)
"$JAVA_HOME/bin/javac" -Xlint:all -Werror --release 17 \
    -cp "${ANDROID_JAR:-$ANDROID_HOME/platforms/android-37.0/android.jar}" -d "$out" "${sources[@]}"
"$JAVA_HOME/bin/java" -cp "$out" com.nhlocal.singlesorter.platform.ReadGuardsTest
