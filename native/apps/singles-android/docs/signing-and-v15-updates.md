# Version 15 Android signing and updates

Version 15 is the next Singles Sorter release, with the same Android package
`com.nhlocal.singlesorter`, product name, original recognition resources and
sorting rules. The initial channel is `15.0.0-beta.1`, version code `1500001`.
`release.json` pins the permanent signing certificate and version identity.

## Permanent key recovery

The permanent RSA 4096 key was created on 2026-10-26 and backed up with its alias
and passwords in `recovery.json`, beside `singles-sorter-android-v15.p12`:

- Primary: `C:\Users\me\Documents\Codex\signing\Singles-Sorter`
- OneDrive: `C:\Users\me\OneDrive\מסמכים\Signing\Singles-Sorter`
- Repository Actions secrets: `ANDROID_KEYSTORE_BASE64`, `ANDROID_KEY_ALIAS`,
  `ANDROID_STORE_PASSWORD`, `ANDROID_KEY_PASSWORD`

The two local copies were verified byte for byte. OneDrive sync completion is
managed by OneDrive and is separate from the verified local backup. GitHub
confirmed the secret uploads. Secrets cannot be read back through the API; use
either local recovery copy to rebuild or repopulate repository secrets.

Never regenerate this key for a routine build. `sign_apk.py` refuses release
output unless its certificate matches `release.json`. Do not commit private key
files or passwords. The public certificate fingerprint is safe to track.

## Transition from 14.0

The downloaded official `v14.0` APK has package `com.nhlocal.singlesorter`,
version code `1`, and certificate SHA256
`b67f10fc6dea13396b722e3c8673c79fc483a98325e0255103eb171011491c19`.
It was signed with a different Android Debug key. Repository and environment
secrets did not contain a signing identity; the local debug key also differs.
The old private key cannot be recovered from the APK's public certificate.

The owner authorized a new permanent key. Therefore **14.0 cannot be updated in
place by this APK**. Before removing 14.0, export its personal singer list and
record settings. Keep music in the shared library and back it up separately.
Install 15.0 after removing 14.0, import the exported CSV and select source and
destination folders again. No automatic migration of app-private Flet data is
claimed. No installer should silently remove the old app.

All subsequent 15.x beta/stable APKs must keep this package and certificate and
increase `android_version_code`. Stable 15.0 must use a higher code than every
published beta. Android 10/API29 is the minimum for the current provider contract;
14.0 supported API23, so older devices retain their existing version.

## Build modes

`build_apk.py` packages verified original models/licenses, Android-mode UI assets
and product JNI libraries using SDK Build Tools 35 and JDK17 or later, on Windows
or Linux. `build-native.sh` remains responsible for rebuilding the JNI libraries.
Set the same input variables documented in `build-preview.sh`, plus
`SINGLES_SIGNING_MODE` (`qa` or `release`), `SINGLES_KEYSTORE`, `SINGLES_KEY_ALIAS`,
`SINGLES_STORE_PASSWORD`, and `SINGLES_KEY_PASSWORD`.

QA builds explicitly enable WebView inspection and have a `-qa.apk` suffix.
Release builds do not enable debugging. Each output has a `.signing.json` receipt
with the APK hash, certificate and legacy signature compatibility. QA receipts
must never be presented as release qualification. The compiled UI and both ABIs
must be tested before promotion from beta to stable.

File access uses the system Storage Access Framework folder picker with retained
read/write grants. Broad storage permissions are unnecessary. Only the system
local external-storage provider is admitted for mutation; cloud providers can
remain read-only. Selecting the root of internal storage or protected Android
directories is restricted by Android; select a music subfolder instead.
