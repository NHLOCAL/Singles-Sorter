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

"$JAVA_HOME/bin/java" -cp "$OUT" com.nhlocal.singlesorter.app.ProjectLinksTest

"$JAVA_HOME/bin/java" -cp "$OUT" com.nhlocal.singlesorter.app.PlanPageSelectionTest
"$JAVA_HOME/bin/java" -cp "$OUT" com.nhlocal.singlesorter.app.LicensePathsTest
"$JAVA_HOME/bin/java" -cp "$OUT" com.nhlocal.singlesorter.app.ProviderNamesTest

"$JAVA_HOME/bin/java" -cp "$OUT" com.nhlocal.singlesorter.app.CheckedJournalCommitTest

"$JAVA_HOME/bin/java" -cp "$OUT" com.nhlocal.singlesorter.app.TargetFingerprintTest

"$JAVA_HOME/bin/java" -cp "$OUT" com.nhlocal.singlesorter.app.ExecutionAttemptTest

"$JAVA_HOME/bin/java" -cp "$OUT" com.nhlocal.singlesorter.app.ExecutionCursorTest

"$JAVA_HOME/bin/java" -cp "$OUT" com.nhlocal.singlesorter.app.LifecycleIoTest

"$JAVA_HOME/bin/java" -cp "$OUT" com.nhlocal.singlesorter.app.LocalWriteGrantPolicyTest

"$JAVA_HOME/bin/java" -cp "$OUT" com.nhlocal.singlesorter.app.LifecycleAcceptanceTest

"$JAVA_HOME/bin/java" -cp "$OUT" com.nhlocal.singlesorter.app.JournalExportBoundsTest
"$JAVA_HOME/bin/java" -cp "$OUT" com.nhlocal.singlesorter.app.DiagnosticReadGateTest

"$JAVA_HOME/bin/java" -cp "$OUT" com.nhlocal.singlesorter.app.ExecutionOutcomesTest

"$JAVA_HOME/bin/java" -cp "$OUT" com.nhlocal.singlesorter.app.RecoveryAcknowledgmentsTest

"$JAVA_HOME/bin/java" -cp "$OUT" com.nhlocal.singlesorter.app.RecoveryScopePolicyTest

"$JAVA_HOME/bin/java" -cp "$OUT" com.nhlocal.singlesorter.app.AncestryResultTest

"$JAVA_HOME/bin/java" -cp "$OUT" com.nhlocal.singlesorter.app.RecoveryContentHashTest

"$JAVA_HOME/bin/java" -cp "$OUT" com.nhlocal.singlesorter.app.RecoveryIntentLookupTest

"$JAVA_HOME/bin/java" -cp "$OUT" com.nhlocal.singlesorter.app.RecoveryOriginalComparisonTest

"$JAVA_HOME/bin/java" -cp "$OUT" com.nhlocal.singlesorter.app.RecoveryAcceptancePolicyTest
"$JAVA_HOME/bin/java" -cp "$OUT" com.nhlocal.singlesorter.app.RecoveryDisplayTest

"$JAVA_HOME/bin/java" -cp "$OUT" com.nhlocal.singlesorter.app.PreviewGenerationTest
