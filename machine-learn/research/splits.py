"""Deterministic NEW research train/development split with transitive grouping.

Never reconstruct a previous split with this code. It deliberately creates no
test split. Holdout exclusions are supplied as IDs/hashes without reading any
sealed records. Results cannot prove an old model never saw the source data.
"""
from dataclasses import asdict
import hashlib
import json
import math
import unicodedata
from typing import Sequence

from evaluation import Record, validate_records


def canonical_bytes(value) -> bytes:
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":"),
                      allow_nan=False).encode("utf-8")


def digest(value) -> str:
    return hashlib.sha256(canonical_bytes(value)).hexdigest()


def normalized_text_key(text: str) -> str:
    """Grouping only: NEVER use this normalized string as annotated model input."""
    normalized = " ".join(unicodedata.normalize("NFC", text).casefold().split())
    return digest(["normalized-text-v1", normalized])


def group_key_hash(key: str) -> str:
    return digest(["declared-group-v1", key])


def _record_value(record):
    return {"record_id": record.record_id, "text": record.text,
            "spans": [asdict(s) for s in sorted(record.spans)],
            "group_keys": sorted(record.group_keys)}


def _validated_hashes(values, name):
    result = set(values)
    if any(not isinstance(v, str) or len(v) != 64
           or any(c not in "0123456789abcdef" for c in v) for v in result):
        raise ValueError(f"{name} must contain lowercase SHA256 digests")
    return result


def build_development_split(
    records: Sequence[Record], *, dataset_id: str, source_reference: str,
    seed: str, development_fraction: float = 0.1,
    forbidden_record_ids=(), forbidden_text_hashes=(), forbidden_group_hashes=(),
) -> dict:
    validate_records(records)
    if not records:
        raise ValueError("a research split requires records")
    if any(not isinstance(x, str) or not x for x in (dataset_id, source_reference, seed)):
        raise ValueError("explicit new dataset ID, source reference and split seed are required")
    if (type(development_fraction) not in (int, float)
            or not math.isfinite(development_fraction) or not 0 < development_fraction < 1):
        raise ValueError("development fraction must be between zero and one")
    forbidden_ids = set(forbidden_record_ids)
    if any(not isinstance(x, str) or not x for x in forbidden_ids):
        raise ValueError("forbidden record IDs must be nonempty strings")
    forbidden_text = _validated_hashes(forbidden_text_hashes, "forbidden text")
    forbidden_groups = _validated_hashes(forbidden_group_hashes, "forbidden groups")
    # Stable ordering makes the manifest independent of the incoming file order.
    rows = sorted(records, key=lambda r: r.record_id)
    parent = list(range(len(rows)))

    def root(i):
        while parent[i] != i:
            parent[i] = parent[parent[i]]
            i = parent[i]
        return i

    def union(a, b):
        a, b = root(a), root(b)
        parent[max(a, b)] = min(a, b)

    owners = {}
    exact_annotations = {}
    tokens = []
    for i, row in enumerate(rows):
        text_hash = normalized_text_key(row.text)
        declared_hashes = {group_key_hash(k) for k in row.group_keys}
        if (row.record_id in forbidden_ids or text_hash in forbidden_text
                or declared_hashes & forbidden_groups):
            raise ValueError("input intersects explicit held-out/exposure exclusions")
        annotations = tuple(sorted(row.spans))
        if row.text in exact_annotations and exact_annotations[row.text] != annotations:
            raise ValueError("conflicting annotations for identical raw text")
        exact_annotations[row.text] = annotations
        row_tokens = {"text:" + text_hash} | {"group:" + h for h in declared_hashes}
        tokens.append(row_tokens)
        for token in row_tokens:
            if token in owners:
                union(i, owners[token])
            else:
                owners[token] = i
    components = {}
    for i in range(len(rows)):
        components.setdefault(root(i), []).append(i)
    membership = {"train": [], "development": []}
    group_receipts = []
    for indices in components.values():
        component_tokens = sorted(set().union(*(tokens[i] for i in indices)))
        component_hash = digest(component_tokens)
        assignment = int(digest(["split-v1", seed, component_hash]), 16)
        numerator, denominator = float(development_fraction).as_integer_ratio()
        role = "development" if assignment * denominator < numerator * (1 << 256) else "train"
        ids = sorted(rows[i].record_id for i in indices)
        membership[role].extend(ids)
        group_receipts.append({"group_sha256": component_hash, "role": role, "record_ids": ids})
    for role in membership:
        membership[role].sort()
    manifest = {
        "schema": "new-research-development-split-v1",
        "dataset_id": dataset_id, "source_reference": source_reference,
        "unicode_data_version": unicodedata.unidata_version,
        "historical_continuation": False, "sealed_test_created_or_read": False,
        "records_sha256": digest([_record_value(r) for r in rows]),
        "record_count": len(rows), "seed": seed,
        "requested_development_fraction": development_fraction,
        "actual_counts": {role: len(ids) for role, ids in membership.items()},
        "membership": membership,
        "groups": sorted(group_receipts, key=lambda g: g["group_sha256"]),
        "exclusion_registry_sha256": digest({"ids": sorted(forbidden_ids),
            "text_hashes": sorted(forbidden_text), "group_hashes": sorted(forbidden_groups)}),
        "scope": "Supplied identities and normalized duplicate texts only; unknown exposure is not ruled out.",
    }
    manifest["manifest_sha256"] = digest(manifest)
    return manifest


def assert_no_leakage(train: Sequence[Record], development: Sequence[Record]) -> None:
    """Read only already-admitted train/development inputs, never a test file."""
    validate_records(train)
    validate_records(development)
    if {r.record_id for r in train} & {r.record_id for r in development}:
        raise ValueError("record ID leakage")
    if {normalized_text_key(r.text) for r in train} & {normalized_text_key(r.text) for r in development}:
        raise ValueError("normalized text leakage")
    left = {k for r in train for k in r.group_keys}
    right = {k for r in development for k in r.group_keys}
    if left & right:
        raise ValueError("declared identity/group leakage")


def validate_manifest(manifest: dict, records: Sequence[Record]) -> None:
    """Verify immutable content and membership before consuming a saved split."""
    if not isinstance(manifest, dict) or manifest.get("schema") != "new-research-development-split-v1":
        raise ValueError("not a new-research development manifest")
    payload = {k: v for k, v in manifest.items() if k != "manifest_sha256"}
    if digest(payload) != manifest.get("manifest_sha256"):
        raise ValueError("manifest digest mismatch")
    if manifest.get("unicode_data_version") != unicodedata.unidata_version:
        raise ValueError("Unicode normalization version differs from frozen split")
    validate_records(records)
    rows = sorted(records, key=lambda r: r.record_id)
    if digest([_record_value(r) for r in rows]) != manifest.get("records_sha256"):
        raise ValueError("record contents differ from frozen manifest")
    membership = manifest.get("membership")
    if not isinstance(membership, dict) or set(membership) != {"train", "development"}:
        raise ValueError("only train/development membership is supported")
    expected = {r.record_id for r in records}
    ids = membership["train"] + membership["development"]
    if len(ids) != len(set(ids)) or set(ids) != expected or len(ids) != manifest.get("record_count"):
        raise ValueError("manifest membership is incomplete or duplicated")
    actual = {role: len(membership[role]) for role in membership}
    if actual != manifest.get("actual_counts"):
        raise ValueError("manifest counts mismatch")
    lookup = {r.record_id: r for r in records}
    assert_no_leakage([lookup[i] for i in membership["train"]],
                      [lookup[i] for i in membership["development"]])
    rebuilt = build_development_split(rows, dataset_id=manifest["dataset_id"],
        source_reference=manifest["source_reference"], seed=manifest["seed"],
        development_fraction=manifest["requested_development_fraction"])
    if rebuilt["membership"] != membership or rebuilt["groups"] != manifest.get("groups"):
        raise ValueError("membership/group receipts differ from deterministic assignment")
    if manifest.get("historical_continuation") is not False or manifest.get("sealed_test_created_or_read") is not False:
        raise ValueError("invalid research scope")
