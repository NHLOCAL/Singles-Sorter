# Releasing the native application (version 15 and later)

`release-v15.yml` builds and accepts every product from one tag, then creates a **draft** GitHub release.

1. Set the same version in `release.json`, `native/Cargo.toml`, `native/apps/singles-ui/package.json` (and its lock),
   `src-tauri/Cargo.toml`, `tauri.conf.json` and `AndroidManifest.xml`. Increase `android_version_code`.
2. Update the three `Cargo.lock` files and the frontend hashes in `native/packaging/licenses/reviewed-desktop-frontend-notices.json`.
3. Confirm the PR checks are green, merge, then push `vX.Y.Z` from `main`.
4. When the workflow ends, review the draft and its `SHA256SUMS.txt`, then publish it. Publishing notifies version 14 users.

A `workflow_dispatch` run produces the same assets as a workflow artifact without creating a release.

The sections below cover the legacy Python package on PyPI, which stays on the 14.x line.

---

# Releasing `singlesorter`

This project uses **SemVer** (`MAJOR.MINOR.PATCH`) and publishes from Git tags in the format `vX.Y.Z`.

## 1) Prepare the release

1. Update `version` in `pyproject.toml` to the target SemVer.
2. Ensure CI is green on `main`.
3. Build and validate locally:

```bash
python -m pip install --upgrade pip
pip install .[dev]
ruff check src tests
pytest -q
python -m build
twine check dist/*
```

## 2) Initial setup validation with TestPyPI (recommended before first real publish)

Create a TestPyPI token and export it as `TEST_PYPI_API_TOKEN`, then run:

```bash
python -m pip install --upgrade pip
pip install build twine
python -m build
twine check dist/*
TWINE_USERNAME=__token__ TWINE_PASSWORD=$TEST_PYPI_API_TOKEN \
  twine upload --repository-url https://test.pypi.org/legacy/ dist/*
python -m venv .venv-testpypi
. .venv-testpypi/bin/activate
pip install --index-url https://test.pypi.org/simple/ --extra-index-url https://pypi.org/simple singlesorter
singlesorter --help
```

## 3) Configure secure publishing

Preferred: **PyPI Trusted Publishing (OIDC)**

1. In PyPI project settings, add a trusted publisher for this repo/workflow:
   - Repository: `NHLOCAL/Singles-Sorter`
   - Workflow: `publish.yml`
   - Environment: _(optional)_
2. No long-lived API key is needed with OIDC.

Fallback: If OIDC cannot be configured, create `PYPI_API_TOKEN` in GitHub repository secrets.

## 4) Cut a release

```bash
git tag vX.Y.Z
git push origin vX.Y.Z
```

The `publish.yml` workflow will:

- build wheel + sdist,
- run `twine check dist/*`,
- publish to PyPI using OIDC (or fallback token).

If any step fails, publishing is blocked.
