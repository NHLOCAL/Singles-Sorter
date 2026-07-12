# Singles Sorter Flet UI Modernization Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deliver a modern, simple RTL Flet GUI for Windows and Android while keeping the default CLI installation free of Flet.

**Architecture:** Keep `singlesorter` framework-independent and add a thin `singlesorter_gui` package with typed state, validation, a background sorting adapter, centralized theme tokens, and focused Flet views. Package Flet only in the `gui` optional dependency and use the existing `MusicSorter` engine from both CLI and GUI.

**Tech Stack:** Python 3.10+, Flet 0.85.3, pytest, Ruff, setuptools, Flet Android tooling.

## Global Constraints

- Windows and Android are equal first-class GUI targets.
- Copy mode is the safe GUI default; the existing CLI flags remain backward compatible.
- `pip install singlesorter` must not install or import Flet.
- Flet is pinned to stable `0.85.3`; pre-release builds are excluded.
- All user-facing GUI copy is concise Hebrew with RTL layout.
- New behavior is introduced test-first.

---

### Task 1: Dependency and package boundaries

**Files:**
- Modify: `pyproject.toml`
- Modify: `requirements.txt`
- Modify: `requirements-apk.txt`
- Create: `tests/test_packaging.py`

**Interfaces:**
- Produces: optional extras `gui`, `ai`, and `dev`; scripts `singlesorter` and `singlesorter-gui`.

- [ ] **Step 1: Write a failing metadata test**

```python
def test_base_dependencies_do_not_include_flet():
    config = tomllib.loads(Path("pyproject.toml").read_text(encoding="utf-8"))
    dependencies = config["project"]["dependencies"]
    assert not any(item.lower().startswith("flet") for item in dependencies)
    assert "flet==0.85.3" in config["project"]["optional-dependencies"]["gui"]
```

- [ ] **Step 2: Run the test and observe failure**

Run: `python -m pytest tests/test_packaging.py -v`
Expected: FAIL because the `gui` extra is absent.

- [ ] **Step 3: Define extras and entry points**

Add `gui = ["flet[all]==0.85.3"]` as recommended by the current Flet installation guide, keep AI dependencies isolated, add `singlesorter-gui = "singlesorter_gui.main:run"`, include `singlesorter_gui*` in package discovery, and make requirements files point at extras rather than duplicate version lists.

- [ ] **Step 4: Verify package tests**

Run: `python -m pytest tests/test_packaging.py -v`
Expected: PASS.

### Task 2: Typed GUI state and validation

**Files:**
- Create: `src/singlesorter_gui/__init__.py`
- Create: `src/singlesorter_gui/state.py`
- Create: `src/singlesorter_gui/validation.py`
- Create: `tests/test_gui_state.py`

**Interfaces:**
- Produces: `SortSettings`, `SortJob`, `SortProgress`, `SortResult`, `validate_job(job) -> tuple[str, ...]`.

- [ ] **Step 1: Write failing state and validation tests**

Tests assert move mode and no internal singles folder are the defaults, settings deserialize safely, missing paths are rejected, identical paths are rejected, and valid existing paths pass.

- [ ] **Step 2: Observe expected failures**

Run: `python -m pytest tests/test_gui_state.py -v`
Expected: import failure because `singlesorter_gui` does not exist.

- [ ] **Step 3: Implement immutable job state and pure validation**

Use dataclasses and `Path.resolve()` without importing Flet. Return concise Hebrew errors rather than raising from validation.

- [ ] **Step 4: Verify state tests**

Run: `python -m pytest tests/test_gui_state.py -v`
Expected: PASS.

### Task 3: Background sorting service and cancellation

**Files:**
- Create: `src/singlesorter_gui/services/__init__.py`
- Create: `src/singlesorter_gui/services/sorting.py`
- Modify: `src/singlesorter/sorter.py`
- Create: `tests/test_sorting_service.py`

**Interfaces:**
- Produces: `CancellationToken`, `SortService.run(job, on_progress) -> SortResult`.
- Extends: `MusicSorter(..., cancel_check: Callable[[], bool] | None = None)`.

- [ ] **Step 1: Write failing adapter tests**

Test settings mapping, structured summaries, progress normalization, and cooperative cancellation using a fake sorter factory.

- [ ] **Step 2: Observe expected failures**

Run: `python -m pytest tests/test_sorting_service.py -v`
Expected: import failure because the service is absent.

- [ ] **Step 3: Implement the framework-independent adapter**

Map `SortJob` to `MusicSorter`, throttle duplicate progress values, capture recoverable failure text, and check cancellation between engine operations.

- [ ] **Step 4: Verify service and engine regressions**

Run: `python -m pytest tests/test_sorting_service.py tests/test_regressions.py -v`
Expected: PASS.

### Task 4: Modern adaptive Flet application

**Files:**
- Create: `src/singlesorter_gui/theme.py`
- Create: `src/singlesorter_gui/app.py`
- Create: `src/singlesorter_gui/main.py`
- Create: `src/singlesorter_gui/views/__init__.py`
- Create: `src/singlesorter_gui/views/home.py`
- Create: `src/singlesorter_gui/views/settings.py`
- Create: `src/singlesorter_gui/views/dialogs.py`
- Create: `tests/test_gui_components.py`
- Modify: `pyproject.toml`

**Interfaces:**
- Produces: `build_theme()`, `HomeView`, `SettingsView`, `SinglesSorterApp`, `main(page)`, `run()`.

- [ ] **Step 1: Write failing component tests**

Tests inspect theme tokens, safe defaults, primary action state, mobile/desktop layout decisions, and the absence of sorter logic in view modules.

- [ ] **Step 2: Observe expected failures**

Run: `python -m pytest tests/test_gui_components.py -v`
Expected: import failure because GUI modules are absent.

- [ ] **Step 3: Implement the blue-and-gold RTL experience**

Build a responsive main flow with two folder cards, one gold primary action, advanced settings in a bottom sheet, review confirmation, background progress, cancellation, and a structured completion dialog. Use current Flet 0.85.3 services and event APIs.

- [ ] **Step 4: Verify GUI component tests**

Run: `python -m pytest tests/test_gui_components.py -v`
Expected: PASS.

### Task 5: Documentation, migration, and full verification

**Files:**
- Modify: `README.md`
- Modify: `RELEASING.md`
- Modify: `src/core/main.py`
- Modify: `pyproject.toml`

**Interfaces:**
- Produces: documented CLI/GUI installs and reproducible desktop/APK/AAB commands.

- [ ] **Step 1: Route the legacy GUI entry point to the new package**

Replace legacy startup with a compatibility import so existing Flet project paths launch `singlesorter_gui.main` while the duplicate UI is no longer active.

- [ ] **Step 2: Document installation and builds**

Document `pip install singlesorter`, `pip install "singlesorter[gui]"`, `singlesorter-gui`, `flet run`, `flet build apk`, and `flet build aab`.

- [ ] **Step 3: Install supported development dependencies**

Run: `python -m pip install -e ".[gui,dev]"`
Expected: Flet 0.85.3 and project editable install complete successfully.

- [ ] **Step 4: Run complete verification**

Run: `python -m pytest -q`
Expected: all tests pass.

Run: `python -m ruff check src tests`
Expected: no errors.

Run: `python -m compileall -q src`
Expected: exit code 0.

Run: `python -m build`
Expected: wheel and source archive build successfully; base wheel metadata has no Flet requirement.

Run: `flet doctor`
Expected: Flet and platform toolchain report without fatal errors.

Run: `flet build apk --module-name singlesorter_gui.main`
Expected: APK build succeeds when Android SDK and Android-compatible dependency wheels are available; otherwise record the exact environmental blocker.
