"""Append-only research evidence receipts, independent of campaign accounting.

This is not a budget allocator, training launcher, ledger reconstruction tool or
automatic stale-lock recovery mechanism. It never refunds or resets a charge.
Only hashes of externally provided evidence are recorded, not its contents.
"""
from pathlib import Path
import hashlib
import json
import os
import re
import uuid

from splits import canonical_bytes

REQUIRED_EVIDENCE = {"dataset_manifest", "training_config", "code_receipt", "campaign_ledger"}
MAX_RECEIPT_BYTES = 64 * 1024


def file_digest(path: Path) -> dict:
    path = Path(path)
    if path.is_symlink() or not path.is_file():
        raise ValueError("evidence must be an existing regular, non-symlink file")
    h = hashlib.sha256()
    total = 0
    with path.open("rb") as stream:
        while chunk := stream.read(65536):
            total += len(chunk)
            if total > 64 * 1024 * 1024:
                raise ValueError("evidence file exceeds 64 MiB")
            h.update(chunk)
    return {"sha256": h.hexdigest(), "bytes": total}


def _directory(path):
    path = Path(path)
    if path.is_symlink() or not path.is_dir():
        raise ValueError("run directory must be a regular directory")
    return path


def inspect_run(directory: Path, *, expected_head_sha256: str | None = None) -> dict:
    """Validate saved chain read-only; interrupted writers are explicitly visible."""
    directory = _directory(directory)
    entries = sorted(directory.iterdir())
    unexpected = [p.name for p in entries if not (
        re.fullmatch(r"\d{8}\.json", p.name) or p.name == ".writer.lock"
        or re.fullmatch(r"\.pending-[a-f0-9]{32}", p.name))]
    if unexpected:
        raise ValueError("unexpected files in receipt directory")
    previous = None
    receipts = []
    for index, path in enumerate(p for p in entries if re.fullmatch(r"\d{8}\.json", p.name)):
        if path.name != f"{index:08d}.json" or path.is_symlink() or not path.is_file():
            raise ValueError("missing, reordered or unsafe receipt")
        if path.stat().st_size > MAX_RECEIPT_BYTES:
            raise ValueError("oversized receipt")
        try:
            raw = path.read_bytes()
            data = json.loads(raw)
        except (ValueError, UnicodeError) as exc:
            raise ValueError("incomplete receipt") from exc
        if type(data) is not dict or set(data) != {"payload", "sha256"}:
            raise ValueError("invalid receipt envelope")
        if raw != canonical_bytes(data) + b"\n":
            raise ValueError("receipt encoding differs from canonical saved bytes")
        payload = data["payload"]
        if type(payload) is not dict or set(payload) != {"schema", "sequence", "previous_sha256", "event", "details"}:
            raise ValueError("invalid receipt payload")
        encoded = canonical_bytes(payload)
        actual = hashlib.sha256(encoded).hexdigest()
        if (data["sha256"] != actual or payload["schema"] != "research-receipt-v1"
                or type(payload["sequence"]) is not int or payload["sequence"] != index
                or payload["previous_sha256"] != previous
                or not isinstance(payload["event"], str) or not payload["event"]
                or type(payload["details"]) is not dict):
            raise ValueError("receipt integrity or chain mismatch")
        receipts.append(data)
        previous = actual
    interrupted = any(p.name.startswith(".") for p in entries)
    if expected_head_sha256 is not None and previous != expected_head_sha256:
        raise ValueError("receipt head differs from externally retained head")
    return {"receipts": receipts, "head_sha256": previous, "count": len(receipts),
            "interrupted_writer": interrupted,
            "head_checked_against_external_reference": expected_head_sha256 is not None,
            "accounting_authority": "external_ledger_only",
            "automatic_training_resume_authorized": False}


def _fsync_directory(directory):
    if os.name != "nt":
        fd = os.open(directory, os.O_RDONLY)
        try:
            os.fsync(fd)
        finally:
            os.close(fd)


def append_receipt(directory: Path, event: str, details: dict, *, expected_head_sha256: str | None) -> dict:
    directory = _directory(directory)
    if not isinstance(event, str) or not event or type(details) is not dict:
        raise ValueError("event and dictionary details are required")
    before = inspect_run(directory)
    if before["head_sha256"] != expected_head_sha256:
        raise ValueError("receipt head changed; reconcile before appending")
    if before["interrupted_writer"]:
        raise ValueError("unresolved writer/pending receipt; never auto-clear ownership")
    nonce = uuid.uuid4().hex
    lock_path = directory / ".writer.lock"
    pending = directory / (".pending-" + nonce)
    try:
        fd = os.open(lock_path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    except FileExistsError as exc:
        raise ValueError("another or interrupted writer owns the run") from exc
    try:
        with os.fdopen(fd, "w", encoding="ascii") as stream:
            stream.write(nonce)
            stream.flush()
            os.fsync(stream.fileno())
        # Re-read while holding the lock: a completed competing writer may have
        # advanced the chain between the initial observation and acquisition.
        state = inspect_run(directory)
        if state["head_sha256"] != expected_head_sha256:
            raise ValueError("receipt head changed while acquiring writer ownership")
        if any(p.name.startswith(".pending-") for p in directory.iterdir()):
            raise ValueError("unresolved pending receipt")
        payload = {"schema": "research-receipt-v1", "sequence": state["count"],
                   "previous_sha256": state["head_sha256"], "event": event, "details": details}
        result = {"payload": payload, "sha256": hashlib.sha256(canonical_bytes(payload)).hexdigest()}
        encoded = canonical_bytes(result) + b"\n"
        if len(encoded) > MAX_RECEIPT_BYTES:
            raise ValueError("receipt too large")
        with pending.open("xb") as stream:
            stream.write(encoded)
            stream.flush()
            os.fsync(stream.fileno())
        # Same-directory hard-link publication is atomic and cannot overwrite an
        # existing receipt. Unsupported filesystems fail closed, no weak fallback.
        os.link(pending, directory / f"{state['count']:08d}.json")
        _fsync_directory(directory)
        pending.unlink()
        return result
    finally:
        # Never remove an unknown writer's lock or an unpublished pending file.
        if lock_path.is_file() and not lock_path.is_symlink() and lock_path.read_text(encoding="ascii") == nonce:
            lock_path.unlink()
            _fsync_directory(directory)


def create_run(directory: Path, *, run_id: str, purpose: str, evidence_paths: dict) -> dict:
    if not run_id or not isinstance(run_id, str) or not purpose or not isinstance(purpose, str):
        raise ValueError("explicit run ID and purpose are required")
    if set(evidence_paths) != REQUIRED_EVIDENCE:
        raise ValueError("dataset/config/code/campaign-ledger evidence must all exist")
    evidence = {name: file_digest(path) for name, path in sorted(evidence_paths.items())}
    directory = Path(directory)
    directory.mkdir(parents=False, exist_ok=False)
    return append_receipt(directory, "created", {
        "run_id": run_id, "purpose": purpose, "evidence": evidence,
        "accounting_policy": "External ledger remains authoritative; no allocation, refund or reset.",
        "training_authorization": "Not established by this receipt.",
    }, expected_head_sha256=None)
