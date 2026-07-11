# Singles Sorter Flet UI Modernization Design

## Objective

Modernize Singles Sorter into a simple, safe, Hebrew-first application for non-technical users on Windows and Android. Upgrade the GUI from Flet 0.28.3 to the latest stable Flet 0.85.3, improve responsiveness and maintainability, and preserve a lightweight CLI installation that does not depend on Flet.

## Product principles

- Copy files by default. Moving files remains available only in advanced settings and requires a clear warning.
- Keep one dominant action on the main screen: "סדר את המוזיקה".
- Hide uncommon choices behind an advanced-settings surface.
- Use short, everyday Hebrew and full right-to-left layout.
- Keep the application responsive during scanning and sorting.
- Preserve existing sorting behavior unless a tested defect requires a change.
- Support Windows and Android as equal first-class targets.

## User experience

### Main screen

The main screen is a short vertical flow:

1. A compact branded header with the application icon, name, and access to help and settings.
2. A source card phrased as "מאיפה לקחת את המוזיקה?" with a large folder-picker action and the selected path.
3. A destination card phrased as "לאן להעתיק אותה?" with the same interaction pattern.
4. A concise safety note confirming that files will be copied and originals will remain in place.
5. One primary button: "סדר את המוזיקה". It stays disabled until both paths are valid.

Desktop uses a centered content column with a comfortable maximum width. Android uses a single-column layout, large touch targets, safe-area spacing, and scrolling where required. The information hierarchy remains the same on both platforms.

### Review and confirmation

Pressing the primary button opens a concise review sheet showing source, destination, copy behavior, and the number of applicable advanced settings. The user can start or return to edit. Move mode presents an explicit destructive-action warning.

### Progress and completion

Sorting runs outside the UI event loop. The progress view shows the current phase, a determinate progress bar when totals are known, processed-file count, and a safe cancel action. Repeated UI updates are throttled to avoid excessive rendering.

Completion shows songs copied, artists identified, folders created, skipped files, and recoverable errors. Error messages explain what the user can do next rather than exposing tracebacks. Detailed diagnostics remain available to logs.

### Advanced settings

Advanced settings are separated from the main flow and grouped by purpose:

- Scan scope: source folder only or include subfolders.
- Destination structure: singles subfolder, alphabetic folders, existing artist folders only.
- Duets: first detected artist or every detected artist.
- File operation: copy by default; move with confirmation.
- Personal singer list management.

Settings use descriptive labels and supporting text. Existing stored values are migrated when possible. Missing values receive safe defaults, with copy mode set to `true`.

## Visual system

The visual direction is a calm music-library utility inspired by the existing icon rather than a literal reproduction of it.

- Primary foundation: deep navy and steel blue.
- Signature accent: warm yellow-gold, reserved for the primary action, progress emphasis, and selected states.
- Surfaces: cool light neutrals in light mode and dark navy surfaces in dark mode.
- Status colors remain semantic and are not replaced by the brand yellow.
- Typography uses the platform-appropriate sans-serif stack with a small, consistent type scale.
- Corners are softly rounded, borders are subtle, and shadows are limited for Android rendering performance.
- Theme values live in one token module and support light and dark modes.

The signature move is a restrained gold progress line that visually connects folder selection, review, and completion.

## Architecture

The GUI becomes a thin application layer over the existing `singlesorter` package. The current duplicate sorter implementation under `src/core` is retired after compatibility tests prove that the package implementation covers the GUI behavior.

Proposed responsibilities:

- `src/singlesorter/`: framework-independent sorting engine and CLI.
- `src/singlesorter_gui/main.py`: Flet entry point and application bootstrap.
- `src/singlesorter_gui/app.py`: application controller and view-state transitions.
- `src/singlesorter_gui/state.py`: typed settings, progress, result, and screen state.
- `src/singlesorter_gui/theme.py`: centralized color, spacing, shape, and typography tokens.
- `src/singlesorter_gui/views/`: focused main, settings, review, progress, result, help, and about views.
- `src/singlesorter_gui/services/`: adapters for sorting jobs, settings persistence, permissions, updates, and file picking.

Views render state and emit user intents. The controller validates input and invokes services. The sorting service adapts `MusicSorter` callbacks into throttled GUI events. No Flet types enter the sorting engine or CLI.

## CLI and dependency packaging

The default package must remain lightweight:

```text
pip install singlesorter
```

installs the sorting engine, CLI, and only their runtime dependencies. It must not install Flet, Flet CLI, desktop runtimes, or Android-specific packages.

Optional dependency groups separate concerns:

```text
pip install "singlesorter[gui]"       # Flet GUI runtime
pip install "singlesorter[ai]"        # optional AI detection
pip install "singlesorter[gui,ai]"    # full desktop development runtime
pip install "singlesorter[dev]"       # tests, lint, and packaging tools
```

The GUI receives its own console entry point, while `singlesorter` remains the CLI entry point. Android build dependencies and permissions remain under Flet build configuration and are not runtime requirements of the base wheel. Dependency versions are defined once in `pyproject.toml`; legacy requirements files are reduced to documented build inputs or removed if no supported workflow needs them.

Flet is pinned to stable `0.85.3` for reproducible desktop and Android builds. Pre-release 0.86 builds are excluded.

## Android behavior

- Use current Flet Android packaging configuration in `pyproject.toml`.
- Build both APK for direct distribution and AAB for Play Store delivery.
- Declare only required storage permissions and use the current cross-platform permission API where possible.
- Keep `flet-permission-handler` only if Flet 0.85.3 does not provide the required storage flow for supported Android versions.
- Validate all binary dependencies for Android wheels before including them in the app bundle.
- Keep AI dependencies optional because native Android wheels and application size may make them unsuitable for the standard Android build.
- Provide actionable permission-denied and inaccessible-folder states.

## Data flow and performance

1. The user selects paths and settings.
2. The controller validates them without starting file operations.
3. The review sheet creates an immutable job request.
4. The sorting service runs the engine in a background worker.
5. Progress events are throttled and applied to view state.
6. Cancellation is cooperative and checked between file operations.
7. The service returns a structured result consumed by the completion view.

Performance improvements focus on eliminating duplicate scans, materializing file lists only when totals are required, caching the singer list once per job, avoiding full-page updates, and keeping expensive metadata work off the UI loop.

## Error handling and safety

- Reject missing, identical, inaccessible, or unwritable folders before starting.
- Never silently fall back from copy to move.
- Keep source files untouched under the default flow.
- Treat individual corrupt or unsupported files as recoverable skips where possible.
- Log technical details with context while presenting short Hebrew messages to users.
- Network update checks must have timeouts and must never block application startup.
- Compare versions semantically rather than lexicographically.

## Testing and verification

Implementation follows test-driven development for new state, validation, adapters, and behavior changes.

- Unit tests: settings defaults and migration, path validation, progress throttling, cancellation, semantic version comparison, and result summaries.
- Engine tests: copy mode, counts, skipped files, duet handling, and compatibility with the retired GUI sorter behavior.
- GUI tests: control-tree and controller-state tests without requiring a visible window where Flet permits it.
- CLI regression: importing and running CLI commands in an environment without Flet installed.
- Dependency regression: verify the base wheel metadata does not require Flet.
- Static checks: Ruff and Python compilation.
- Packaging: build the Python wheel, run Flet desktop validation, run `flet doctor`, and build an Android APK. Build AAB when signing-independent validation is possible.
- Manual smoke tests: Windows desktop sizes, narrow Android viewport or emulator, RTL, light/dark theme, permission denial, cancellation, and successful copy flow.

## Migration and scope

The migration is incremental. First lock existing engine behavior with tests, then introduce state and service boundaries, then replace the UI, and finally remove duplicate legacy GUI logic. Historical files and release artifacts are not broadly reorganized unless they interfere with supported builds.

This work does not add cloud storage, playlists, audio playback, account systems, or new AI features. The focus is a safer and simpler sorting experience, current Flet support, maintainable code, and clean separation between GUI and CLI.
