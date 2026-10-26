"""Check every Rust scope, retain diagnostics, and restore the checked source bytes.

This is a strict CI check, not an auto-fix: a failing original check always fails
this command, even when a diagnostic formatter produces a correction.
"""
from __future__ import annotations

import argparse
import json
from pathlib import Path
import subprocess

SCOPES = (
    ("workspace", "native/Cargo.toml"),
    ("desktop", "native/apps/singles-ui/src-tauri/Cargo.toml"),
    ("android", "native/plugins/original-inference/Cargo.toml"),
)


def invoke(command: list[str], root: Path) -> dict:
    try:
        result = subprocess.run(command, cwd=root, stdout=subprocess.PIPE,
                                stderr=subprocess.STDOUT, timeout=120)
        return {"command": command, "code": result.returncode,
                "output": result.stdout.decode("utf-8", errors="replace")}
    except (OSError, subprocess.TimeoutExpired) as error:
        return {"command": command, "code": -1, "output": str(error)}


def git(root: Path, *args: str) -> bytes:
    return subprocess.check_output(["git", *args], cwd=root)


def collect(root: Path, out: Path, run=invoke) -> int:
    # Do not discard a developer's changes or include them in a proposed patch.
    if git(root, "status", "--porcelain", "--untracked-files=no", "--", "native"):
        raise RuntimeError("Formatter diagnostics require clean tracked native source")
    paths = git(root, "ls-files", "-z", "--", "native").split(b"\0")
    originals = {root / path.decode("utf-8"): (root / path.decode("utf-8")).read_bytes()
                 for path in paths if path.endswith(b".rs")}
    out.mkdir(parents=True, exist_ok=True)
    report = {"source_commit": git(root, "rev-parse", "HEAD").decode().strip(),
              "checks": [], "corrections": [], "source_restored": False}
    failed = False
    try:
        # Check all scopes before running any corrective formatter. A failure in
        # the first scope must not conceal errors in either nested workspace.
        for name, manifest in SCOPES:
            command = ["cargo", "+1.99.0", "fmt", "--manifest-path", manifest, "--all"]
            result = run([*command, "--", "--check"], root)
            report["checks"].append({"scope": name, **result})
            failed |= result["code"] != 0
        if failed:
            for name, manifest in SCOPES:
                command = ["cargo", "+1.99.0", "fmt", "--manifest-path", manifest, "--all"]
                result = run(command, root)
                report["corrections"].append({"scope": name, **result})
    finally:
        try:
            (out / "native-formatting.patch").write_bytes(
                git(root, "diff", "--binary", "--", "native"))
        finally:
            # Restore exact original bytes, including Windows checkout line ends.
            # No reset, clean, broad restore, or untracked-file deletion is used.
            for path, original in originals.items():
                if not path.exists() or path.read_bytes() != original:
                    path.write_bytes(original)
            report["source_restored"] = not bool(
                git(root, "status", "--porcelain", "--untracked-files=no", "--", "native"))
            (out / "commands.json").write_text(
                json.dumps(report, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    for check in report["checks"]:
        print(f'{check["scope"]}: formatter check exit {check["code"]}')
    if not report["source_restored"]:
        raise RuntimeError("Unexpected non-Rust source change; original check cannot continue")
    return int(failed)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", type=Path, required=True)
    args = parser.parse_args()
    return collect(Path(__file__).resolve().parents[3], args.out.resolve())


if __name__ == "__main__":
    raise SystemExit(main())
