#!/usr/bin/env python3
"""Original-only release CLI gate using temporary, authored silent WAVs.

The original Python routing and AI methods are read from the pinned public Git
baseline, not reimplemented here. Python is an oracle/build dependency only.
Never reads a music library or private corpus; never executes a plan.
"""
from __future__ import annotations

import argparse
import contextlib
import csv
import hashlib
import importlib.util
import io
import json
import logging
import os
from pathlib import Path
import pickle
import shutil
import subprocess
import sys
import tempfile
import types
import wave

BASELINE = "78d001aee5ce49628bdffebd34540fa83ba32c12"
BUNDLE_ID = "original-v14-ner-3.7.5"
CLASSIFIER_SHA256 = "ec9b4b50e9308d5f7a27f12bc141389e61a16a622a4faa0cbf864f94a9cb3293"
ASSETS = ("manifest.json", "tensors.f32", "tokenizer.json",
          "classifier-v14.weights.bin", "classifier-v14.vocab.txt")
NATIVE = Path(__file__).resolve().parents[2]


def require(condition, message):
    if not condition:
        raise AssertionError(message)


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def manifest(root):
    """Include names, types, bytes, permissions and mtimes; exclude read atime."""
    result = []
    for path in sorted([root, *root.rglob("*")]):
        info = path.lstat()
        result.append((str(path.relative_to(root)), info.st_mode, info.st_size,
                       info.st_mtime_ns, digest(path) if path.is_file() else None))
    return result


def authored_wav(path, artist="", title=""):
    from mutagen.id3 import TIT2, TPE1
    from mutagen.wave import WAVE

    with wave.open(str(path), "wb") as output:
        output.setnchannels(1)
        output.setsampwidth(2)
        output.setframerate(8000)
        output.writeframes(b"\0\0" * 800)
    if artist or title:
        audio = WAVE(path)
        audio.add_tags()
        if artist:
            audio.tags.add(TPE1(encoding=3, text=[artist]))
        if title:
            audio.tags.add(TIT2(encoding=3, text=[title]))
        audio.save()
    # Confirm fixture authoring independently, before either implementation runs.
    import music_tag
    metadata = music_tag.load_file(str(path))
    require(metadata["artist"].value == artist, "authored artist did not round-trip")
    require(metadata["title"].value == title, "authored title did not round-trip")


def git_bytes(baseline, path):
    return subprocess.check_output(["git", "-C", str(baseline), "show", f"{BASELINE}:{path}"])


@contextlib.contextmanager
def original_oracle(baseline, bundle, temporary):
    """Run unchanged release methods, with explicit already-verified model paths."""
    import numpy
    import sklearn
    import spacy
    import thinc
    import unicodedata

    require((sys.version_info[:2], unicodedata.unidata_version, numpy.__version__,
             sklearn.__version__, spacy.__version__, thinc.__version__) ==
            ((3, 12), "15.0.0", "1.26.4", "1.5.1", "3.7.5", "8.2.5"),
            "wrong combined original oracle runtime")
    head = subprocess.check_output(["git", "-C", str(baseline), "rev-parse", "HEAD"],
                                   text=True).strip()
    require(head == BASELINE, "oracle baseline checkout is not pinned v14")
    code = temporary / "oracle-code"
    code.mkdir()
    sources = {}
    for name in ("check_name.py", "jibrish_to_hebrew.py", "singles_sorter_v5.py", "ai_models.py"):
        data = git_bytes(baseline, f"src/core/{name}")
        (code / name).write_bytes(data)
        sources[name] = hashlib.sha256(data).hexdigest()
    aliases = git_bytes(baseline, "src/core/app/singer-list.csv")
    require(aliases == (NATIVE / "tests/fixtures/full_contract/released-singer-list.csv").read_bytes(),
            "CLI bundled aliases differ from original release")
    rows = list(csv.reader(io.StringIO(aliases.decode("utf-8-sig"))))
    model_bytes = (bundle / "original-release.pkl").read_bytes()
    require(hashlib.sha256(model_bytes).hexdigest() == CLASSIFIER_SHA256,
            "refusing unauthenticated pickle")
    classifier = pickle.loads(model_bytes)
    # Verify every original NER source byte before deserializing spaCy assets.
    lock = json.loads((NATIVE / "probes/released-ner/released-ner-export/release-lock.json")
                      .read_text(encoding="utf-8"))
    model = baseline / "src/core/models/singer_ner_he"
    for name, expected in lock["source_files"].items():
        require(digest(model / name) == expected, f"original NER source changed: {name}")
    ner = spacy.load(model)
    previous = Path.cwd()
    sys.path.insert(0, str(code))
    try:
        os.chdir(code)  # The original relative-path discovery must not find models.
        with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
            import singles_sorter_v5 as released
            spec = importlib.util.spec_from_file_location("cli_oracle_ai_models", code / "ai_models.py")
            ai_module = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(ai_module)
        logger = logging.getLogger("original-cli-oracle")
        logger.handlers = [logging.NullHandler()]
        logger.propagate = False
        ai = ai_module.AIModels.__new__(ai_module.AIModels)
        ai.logger, ai.nlp, ai.sklearn_model = logger, ner, classifier
        trace = []

        def classify(self, text):
            value = ai_module.AIModels.verify_artist_with_sklearn(self, text)
            trace.append({"kind": "classifier", "text": text, "accepted": bool(value)})
            return value

        def recognize(self, text):
            trace.append({"kind": "ner", "text": text})
            return ai_module.AIModels.process_with_ner(self, text)

        ai.verify_artist_with_sklearn = types.MethodType(classify, ai)
        ai.process_with_ner = types.MethodType(recognize, ai)
        sorter = released.MusicSorter.__new__(released.MusicSorter)
        sorter.logger, sorter.singer_list, sorter.ai_models = logger, rows, ai

        def predict(path, enabled=True):
            trace.clear()
            sorter.ai_models = ai if enabled else None
            released.ai_invalid = not enabled  # Both pinned original models are explicitly loaded.
            result = list(sorter.artists_from_song(path) or [])
            return result, list(trace)

        yield predict, sources
    finally:
        os.chdir(previous)
        sys.path.remove(str(code))


class Gate:
    def __init__(self, binary, bundle, root):
        self.bundle, self.root, self.count = bundle, root, 0
        executable_dir = root / "תוכנת בדיקה"
        executable_dir.mkdir()
        self.binary = executable_dir / ("מסדר סינגלים" + binary.suffix)
        shutil.copy2(binary, self.binary)
        self.cases = []

    def tree(self, name, files):
        root = self.root / name
        for directory in (root, root / "מקור", root / "יעד", root / "unrelated-cwd", root / "preferences"):
            directory.mkdir()
        (root / "יעד/existing-sentinel.bin").write_bytes(b"untouched target\0\xff")
        (root / "preferences/settings.json").write_bytes(b'{"owned":"unchanged"}\n')
        paths = []
        for filename, artist, title in files:
            path = root / "מקור" / filename
            authored_wav(path, artist, title)
            paths.append(path)
        return root, paths

    def request(self, root, *, enabled=True, model=None, limits=None, mode="Sort"):
        profile = {"bundle_id": BUNDLE_ID, "model_directory": str(model or self.bundle)}
        if limits is not None:
            profile["limits"] = limits
        return {"schema_version": 2, "mode": mode, "source": str(root / "מקור"),
                "target": str(root / "יעד") if mode == "Sort" else None,
                "settings": {}, "ai_profile": {"Configured": profile} if enabled else "Disabled",
                "personal_aliases": None}

    def invoke(self, root, request, *, success=True, inferred=None, error_code=None):
        before = manifest(root)
        process = subprocess.run([str(self.binary), "plan-files", "--json"],
                                 input=json.dumps(request, ensure_ascii=False).encode("utf-8"),
                                 cwd=root / "unrelated-cwd", capture_output=True, timeout=120)
        require(manifest(root) == before, "CLI changed source, target, preferences or unrelated CWD")
        require(process.stdout.endswith(b"\n"), "CLI response must end with a newline")
        response = json.loads(process.stdout.decode("utf-8"))
        require(response["schema_version"] == 2, "wrong CLI protocol")
        result = response["result"]
        require(result["executable"] is False, "preview must never become executable")
        require(type(result["inference_executed"]) is bool, "inference status must be a bool")
        if inferred is not None:
            require(result["inference_executed"] is inferred, ("wrong inference status", response))
        if success:
            require(process.returncode == 0 and response["status"] == "ok" and response["error"] is None,
                    ("CLI failed", process.returncode, response, process.stderr.decode("utf-8")))
            require(process.stderr == b"", "successful protocol must not print diagnostics")
            require(result["plan"]["executable"] is False, "plan must stay non-executable")
        else:
            require(process.returncode != 0 and response["status"] == "error", ("unsafe fallback", response))
            require(result["plan"] is None, "fatal error leaked a partial plan")
            require(response["error"]["code"] in result["blockers"], "missing fatal blocker")
            if error_code is not None:
                require(response["error"]["code"] == error_code, ("wrong fatal error", response))
            require(bool(process.stderr), "error lacks stderr diagnostic")
        self.count += 1
        return response

    def artists(self, response, path):
        singles = [d["Single"] for d in response["result"]["plan"]["decisions"] if "Single" in d]
        matches = [d for d in singles if d["read"]["source"]["name"] == path.name]
        require(len(matches) == 1, ("missing or duplicate single decision", path.name, singles))
        return matches[0]["artists"]


def check(binary, bundle, baseline, disabled_binary=None):
    with tempfile.TemporaryDirectory(prefix="original-ai-cli-") as temp:
        temporary = Path(temp).resolve()
        gate = Gate(binary.resolve(), bundle.resolve(), temporary)
        bundle_before = {name: digest(bundle / name) for name in ASSETS}
        # Fixed authored examples, not mined datasets. Route assertions below prevent
        # the gate from silently becoming an alias-only test or testing no positives.
        cases = [
            ("alias", "אברהם פריד - בדיקה.wav", "", "", "alias"),
            ("metadata-accept", "owned metadata.wav", "ישראל כהן", "", "classifier"),
            ("metadata-reject", "owned negative.wav", "בדיקת מסלול", "", "reject"),
            ("metadata-reject-fallback", "ישראל כהן - שיר חדש.wav", "בדיקת מסלול", "", "ner"),
            ("filename-ner", "ישראל כהן - שיר חדש.wav", "", "", "ner"),
            ("title-ner", "owned title.wav", "", "ישראל כהן - שיר חדש", "title"),
            ("ner-classifier-reject", "בדיקת מסלול - שיר חדש.wav", "", "", "ner-reject"),
            ("duet", "ישראל כהן וחיים לוי - שיר חדש.wav", "", "", "ner"),
            ("unicode-title", "owned unicode.wav", "", "🎵 ישראל כהן - שיר חדש", "title"),
        ]
        with original_oracle(baseline.resolve(), bundle.resolve(), temporary) as (oracle, sources):
            for name, filename, artist, title, route in cases:
                root, paths = gate.tree(name, [(filename, artist, title)])
                before = manifest(root)
                expected, trace = oracle(paths[0])
                require(manifest(root) == before, "original read-only oracle changed fixture bytes")
                kinds = [t["kind"] for t in trace]
                if route == "alias":
                    require(bool(expected) and not trace, "alias fixture must bypass inference")
                elif route == "classifier":
                    require(expected == [artist] and kinds == ["classifier"], "metadata route was not exercised")
                elif route == "reject":
                    require(not expected and kinds[0] == "classifier" and not trace[0]["accepted"],
                            "metadata reject fixture stopped testing rejection")
                elif route in ("ner", "title"):
                    require(bool(expected) and "ner" in kinds and "classifier" in kinds,
                            "NER fixture must execute both original models")
                    if route == "title":
                        require(sum(k == "ner" for k in kinds) == 2, "title fallback was not exercised")
                elif route == "ner-reject":
                    require(not expected and "ner" in kinds and any(t.get("accepted") is False for t in trace),
                            "NER span classifier rejection was not exercised")
                request = gate.request(root)
                request["settings"]["duet_mode"] = True
                response = gate.invoke(root, request, inferred=bool(trace))
                require(gate.artists(response, paths[0]) == expected,
                        ("original combined routing mismatch", name, expected, response))
                observations = response["result"]["plan"]["metadata_observations"]
                opened = [o["Open"] for o in observations if "Open" in o
                          and o["Open"]["source"]["name"] == filename]
                require(opened and all("Ok" in o["result"] for o in opened),
                        "actual native WAV metadata reader did not open fixture")
                for field, value in (("Title", title), ("Artist", artist)):
                    observed = [o["Get"] for o in observations if "Get" in o
                                and o["Get"]["source"]["name"] == filename
                                and o["Get"]["field"] == field]
                    if field == "Title" or artist:
                        require(observed and all(o["result"].get("Ok", {}).get("value") ==
                                                 {"Ok": {"Text": value}} for o in observed),
                                ("native metadata differs from authored original input", name, field, observed))
                # Explicit disabled mode still uses real metadata, without model use.
                disabled_expected, disabled_trace = oracle(paths[0], enabled=False)
                disabled = gate.invoke(root, gate.request(root, enabled=False), inferred=False)
                require(not disabled_trace and gate.artists(disabled, paths[0]) == disabled_expected,
                        "disabled routing changed")
                gate.cases.append({"case": name, "artists": expected, "original_calls": trace})

        if disabled_binary is not None:
            # Compile-time opt-in is independent from a caller requesting a profile.
            root, _ = gate.tree("compile-time-off", [("owned.wav", "ישראל כהן", "")])
            configured_binary = gate.binary
            gate.binary = disabled_binary.resolve()
            gate.invoke(root, gate.request(root), success=False, inferred=False)
            gate.invoke(root, gate.request(root, enabled=False), inferred=False)
            gate.binary = configured_binary

        # The plan binds verified resources and effective runtime limits, never
        # provenance or availability assertions supplied by a caller.
        root, _ = gate.tree("verified-binding", [("owned.wav", "ישראל כהן", "")])
        first = gate.invoke(root, gate.request(root), inferred=True)["result"]["plan"]
        expected_resources = {
            "original-ner-tensors:sha256:" + digest(bundle / "tensors.f32"),
            "original-ner-tokenizer:sha256:" + digest(bundle / "tokenizer.json"),
            "original-secondary-weights:sha256:" + digest(bundle / "classifier-v14.weights.bin"),
            "original-secondary-vocabulary:sha256:" + digest(bundle / "classifier-v14.vocab.txt"),
        }
        require(expected_resources <= set(first["request"]["resource_ids"]),
                "plan does not bind every verified original model resource")
        adapters = first["request"]["adapter_versions"]
        require("original-model-source:" + BASELINE in adapters,
                "plan lacks authenticated original source identity")
        require(any(v.startswith("original-native-readonly-v1:") and
                    "pcre2-10.46-static32-no-jit" in v for v in adapters),
                "plan lacks original-only runtime identity")
        tightened = gate.invoke(root, gate.request(root, limits={"max_inference_calls": 9}),
                                inferred=True)["result"]["plan"]
        require(first["binding_sha256"] != tightened["binding_sha256"],
                "effective inference limits are absent from plan binding")
        require(first["request"]["resource_ids"] == tightened["request"]["resource_ids"],
                "tightening job limits changed model identity")
        for field in ("resource_ids", "adapter_versions"):
            request = gate.request(root)
            request[field] = ["forged-by-caller"]
            gate.invoke(root, request, success=False, inferred=False, error_code="INVALID_INPUT")
        request = gate.request(root)
        request["ai_profile"]["Configured"]["available"] = True
        gate.invoke(root, request, success=False, inferred=False, error_code="INVALID_INPUT")

        # Configured FixNames does not invoke inference, even with valid loaded assets.
        root, _ = gate.tree("fix-names", [("owned_file.wav", "ישראל כהן", "")])
        gate.invoke(root, gate.request(root, mode="FixNames"), inferred=False)

        root, _ = gate.tree("bad-profile", [("owned.wav", "ישראל כהן", "")])
        request = gate.request(root)
        request["ai_profile"]["Configured"]["bundle_id"] = "candidate-not-original"
        gate.invoke(root, request, success=False, inferred=False)
        request = gate.request(root)
        del request["ai_profile"]["Configured"]["model_directory"]
        gate.invoke(root, request, success=False, inferred=False)
        gate.invoke(root, gate.request(root, model=temporary / "missing-model"), success=False, inferred=False)
        request = gate.request(root)
        request["ai_profile"]["Configured"]["model_directory"] = "relative-model"
        gate.invoke(root, request, success=False, inferred=False)

        # Each required part independently missing and corrupt, with all other parts original.
        for filename in ASSETS:
            for mutation in ("missing", "corrupt"):
                model = temporary / (mutation + "-" + filename)
                model.mkdir()
                for asset in ASSETS:
                    if asset != filename or mutation != "missing":
                        data = (bundle / asset).read_bytes()
                        if asset == filename:
                            data = bytes([data[0] ^ 1]) + data[1:]
                        (model / asset).write_bytes(data)
                prior = manifest(model)
                gate.invoke(root, gate.request(root, model=model), success=False, inferred=False)
                require(manifest(model) == prior, "loader wrote model assets")

        # A bounded inference invocation may fail before arithmetic; it was attempted.
        root, _ = gate.tree("classifier-limit", [("owned.wav", "ישראל כהן", "")])
        gate.invoke(root, gate.request(root, limits={"classifier_input_bytes": 1}), success=False, inferred=True,
                    error_code="PLANNING_RESOURCE_LIMIT")
        root, _ = gate.tree("ner-limit", [("ישראל כהן - שיר חדש.wav", "", "")])
        gate.invoke(root, gate.request(root, limits={"ner_input_bytes": 1}), success=False, inferred=True,
                    error_code="PLANNING_RESOURCE_LIMIT")

        gate.invoke(root, gate.request(root, limits={"max_inference_calls": 0}),
                    success=False, inferred=False, error_code="PLANNING_RESOURCE_LIMIT")

        # One real classifier prediction succeeds; the second request exhausts the
        # job budget. Nothing may escape as an executable or partial preview.
        root, _ = gate.tree("partial-plan-forbidden", [
            ("one.wav", "ישראל כהן", ""), ("two.wav", "חיים לוי", "")])
        gate.invoke(root, gate.request(root, limits={"max_inference_calls": 1}),
                    success=False, inferred=True, error_code="PLANNING_RESOURCE_LIMIT")
        require({name: digest(bundle / name) for name in ASSETS} == bundle_before, "original model bundle changed")
        return {"result": "PASS", "cli_invocations": gate.count,
                "cli_sha256": digest(binary), "baseline": BASELINE,
                "oracle_source_sha256": sources, "cases": gate.cases,
                "real_audio": "owned generated silent WAV with ID3 metadata",
                "original_models_only": True, "source_target_preferences_unchanged": True,
                "partial_plan_on_fatal_error": False, "deployment_python_required": False,
                "private_corpus_read": False}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", required=True, type=Path)
    parser.add_argument("--disabled-binary", type=Path)
    parser.add_argument("--bundle", required=True, type=Path)
    parser.add_argument("--baseline", required=True, type=Path)
    parser.add_argument("--report", type=Path)
    args = parser.parse_args()
    report = check(args.binary, args.bundle, args.baseline, args.disabled_binary)
    encoded = json.dumps(report, ensure_ascii=False, indent=2) + "\n"
    if args.report:
        args.report.write_bytes(encoded.encode("utf-8"))
    print(encoded, end="")
