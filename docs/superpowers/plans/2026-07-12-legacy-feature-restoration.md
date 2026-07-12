# Legacy GUI Feature Restoration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Restore every relevant user-facing capability from the original GUI in the modern Flet 0.85.3 application and ship verified Android and Windows builds.

**Architecture:** Keep the sorting engine and CLI independent of Flet. Add focused GUI modules for informational content, custom singer-list persistence, release checks, and secondary actions; the application controller only coordinates these services and full-screen adaptive views.

**Tech Stack:** Python 3.10+, Flet 0.85.3, standard-library CSV/JSON/HTTP APIs, pytest, Ruff, Flet Android and Windows builders.

## Global Constraints

- Preserve the current simple two-folder primary flow and RTL blue/gold design.
- Restore Help, What's New, About, fix-names, personal singer list, update checks, first-run release notes, tips, version and project links.
- Windows and Android must expose equivalent capabilities except file export where the platform picker cannot provide a writable path.
- Base CLI installation must not import or install Flet.
- All new behavior is test-first and packaged resources must work outside the repository checkout.

---

### Task 1: Feature inventory and information center

**Files:**
- Create: `src/singlesorter_gui/content.py`
- Create: `src/singlesorter_gui/views/information.py`
- Create: `src/singlesorter_gui/content/help.md`
- Create: `src/singlesorter_gui/content/whats-new.md`
- Create: `src/singlesorter_gui/content/about.md`
- Modify: `src/singlesorter_gui/views/home.py`
- Modify: `src/singlesorter_gui/app.py`
- Modify: `pyproject.toml`
- Test: `tests/test_gui_information.py`

**Interfaces:**
- Produces: `load_content(name) -> str`, `information_sheet(...) -> ft.BottomSheet`, and a header overflow menu exposing all restored sections.

- [x] **Step 1: Write failing tests for packaged content and menu actions**
- [x] **Step 2: Run `python -m pytest tests/test_gui_information.py -v` and observe missing-module failures**
- [x] **Step 3: Implement packaged Hebrew content, full-screen scrolling presentation, version display, close action and external project links**
- [x] **Step 4: Run the information tests and verify all menu entries are reachable**

### Task 2: Restore the fix-names workflow

**Files:**
- Modify: `src/singlesorter_gui/services/sorting.py`
- Modify: `src/singlesorter_gui/views/home.py`
- Modify: `src/singlesorter_gui/app.py`
- Test: `tests/test_sorting_service.py`
- Test: `tests/test_gui_components.py`

**Interfaces:**
- Produces: `SortService.fix_names(source, on_progress, cancellation) -> SortResult` and a secondary guarded action on the home screen.

- [x] **Step 1: Write failing service and component tests for fix-names**
- [x] **Step 2: Verify failures show the action is absent**
- [x] **Step 3: Implement confirmation, background execution, progress, cancellation and result handling using `MusicSorter.fix_names()`**
- [x] **Step 4: Run service, GUI and engine regression tests**

### Task 3: Restore personal singer-list management

**Files:**
- Create: `src/singlesorter_gui/services/singer_list.py`
- Create: `src/singlesorter_gui/views/singer_list.py`
- Modify: `src/singlesorter_gui/app.py`
- Modify: `src/singlesorter/sorter.py`
- Test: `tests/test_singer_list_service.py`
- Test: `tests/test_gui_components.py`

**Interfaces:**
- Produces: `SingerEntry`, `SingerListStore.load/save/import_csv/export_csv`, and an adaptive editor with add/remove/save/import/export actions.

- [x] **Step 1: Write failing persistence, validation, deduplication and encoding tests**
- [x] **Step 2: Observe expected failures for the missing store**
- [x] **Step 3: Implement an application-data CSV path and point the engine to it through `SINGLESORTER_PERSONAL_LIST`**
- [x] **Step 4: Implement the full-screen editor and platform-aware import/export**
- [x] **Step 5: Run singer-list and GUI regression tests**

### Task 4: Restore updates, first-run notes and tips

**Files:**
- Create: `src/singlesorter_gui/services/updates.py`
- Create: `src/singlesorter_gui/tips.py`
- Modify: `src/singlesorter_gui/app.py`
- Modify: `src/singlesorter_gui/views/home.py`
- Test: `tests/test_update_service.py`
- Test: `tests/test_gui_state.py`

**Interfaces:**
- Produces: `ReleaseInfo`, `check_latest_release(current_version)`, first-run version persistence, non-blocking update status and deterministic tip selection.

- [x] **Step 1: Write failing semantic-version and release-response tests without live network access**
- [x] **Step 2: Observe the missing update service failure**
- [x] **Step 3: Implement a timeout-bounded standard-library GitHub release client and friendly offline behavior**
- [x] **Step 4: Add update badge/dialog, first-run What's New and concise contextual tips without blocking startup**
- [x] **Step 5: Run update, state and GUI tests**

### Task 5: Windows EXE, Android rebuild and completion audit

**Files:**
- Modify: `README.md`
- Modify: `RELEASING.md`
- Modify: `docs/superpowers/plans/2026-07-12-flet-ui-modernization.md`
- Modify: this plan

**Interfaces:**
- Produces: Windows executable bundle, split Android APKs, AAB, wheel/sdist and a feature-parity completion record.

- [x] **Step 1: Run `python -m pytest -q`, Ruff, compileall and `python -m build`**
- [x] **Step 2: Run `flet build windows --no-rich-output` and launch the generated executable**
- [x] **Step 3: Run Android APK/AAB builds and an SDK smoke test of restored menu flows**
- [x] **Step 4: Compare every original menu/action against the new UI and record the result**
- [x] **Step 5: Update documentation, mark verified plan items, commit and push**

## Feature-parity audit

| Original capability | Modern implementation | Verification |
|---|---|---|
| Help | Packaged Hebrew Markdown in a full-screen scrolling sheet | pytest + Android API 35 |
| What's New | Packaged release notes, menu action and once-per-version startup display | pytest + Android API 35 |
| About, version and links | About page, visible version, website/GitHub/releases/issues/Shir Bot links | pytest + packaged-wheel inspection |
| Version updates | Timeout-bounded background GitHub check, badge and release dialog | isolated HTTP tests |
| Advanced settings | Full-screen scrolling settings with fixed save/cancel actions | pytest + Android 12/API 35 |
| Personal singer list | App-data storage, editor, add/remove, UTF-8/CP1255 import and export | pytest + Android API 35 |
| Fix names and metadata | Guarded secondary action, warning, background progress and cancellation | pytest |
| Tips | Non-blocking rotating contextual tips on the home screen | pytest + Android API 35 |
| Theme switch | Light/dark theme action retained in the header | component tests |
| AI model updater | Replaced by versioned optional `ai` package/app releases; in-place mutation was intentionally retired because packaged EXE/APK resources are immutable | packaging tests |

## Verification record

- `python -m pytest -q`: 47 passed.
- `python -m ruff check src tests`: passed.
- `python -m compileall -q src`: passed.
- `python -m build`: wheel and source archive built; restored content confirmed inside the wheel.
- `flet build windows --no-rich-output`: succeeded after repairing a zero-byte official Python runtime download and the 32-bit CMake `System32` redirect; `build/windows/singlesorter.exe` remained active during a 12-second smoke test.
- `flet build apk --split-per-abi --no-rich-output`: succeeded.
- `flet build aab --no-rich-output`: succeeded.
- Android SDK: x86_64 APK installed on `shamor_api35`; first-run What's New, restored menu, Help and personal singer editor verified through the UI hierarchy with no Python traceback.
