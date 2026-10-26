"""Owned synthetic fixtures only. Passing tests is NOT model-quality evidence."""
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from evaluation import Record, Span, evaluate, scores, spacy_predictor, validate_spans
from receipts import append_receipt, create_run, inspect_run
from splits import (assert_no_leakage, build_development_split, canonical_bytes,
                    digest, group_key_hash, normalized_text_key, validate_manifest)
from training_control import DevelopmentStopper, StopConfig


class EvaluationTests(unittest.TestCase):
    def setUp(self):
        self.rows = [Record("1", "אב גד - שיר", (Span(0, 5, "SINGER"), Span(8, 11, "SONG"))),
                     Record("2", "צליל בלבד", ()), Record("3", "אמן - שיר", (Span(0, 3, "SINGER"),))]

    def test_one_prediction_pass_serves_all_labels_and_errors(self):
        calls = []
        def predict(texts):
            calls.append(list(texts))
            yield self.rows[0].spans
            yield (Span(0, 4, "SINGER"),)
            yield ()
        result = evaluate(self.rows, predict, labels=["SINGER", "SONG", "MISC", "ALBUM", "GENRE"])
        self.assertEqual(len(calls), 1)
        self.assertEqual(len(calls[0]), 3)
        self.assertEqual(result["per_label"]["SINGER"], scores(1, 1, 1))
        self.assertEqual(result["micro"], scores(2, 1, 1))
        self.assertEqual(result["false_singer_negative_records"], 1)
        self.assertEqual(result["exact_ordered_artist_records"], 1)
        self.assertNotIn("אב", json.dumps(result, ensure_ascii=False))

    def test_unexpected_label_false_positives_are_not_hidden(self):
        result = evaluate([Record("r", "sound", ())], lambda _: [(Span(0, 5, "UNEXPECTED"),)])
        self.assertEqual(result["micro"]["fp"], 1)

    def test_empty_input_is_defined(self):
        result = evaluate([], lambda _: [])
        self.assertEqual(result["records"], 0)
        self.assertEqual(result["micro"]["f1"], 0)

    def test_missing_prediction_fails(self):
        with self.assertRaises(ValueError):
            evaluate(self.rows, lambda _: [()])

    def test_extra_prediction_fails(self):
        with self.assertRaises(ValueError):
            evaluate([], lambda _: [()])

    def test_duplicate_record_id_fails_before_prediction(self):
        with self.assertRaises(ValueError):
            evaluate([self.rows[0], self.rows[0]], lambda _: self.fail("must not predict"))

    def test_invalid_offsets_fail_instead_of_skipping_gold(self):
        for spans in [(Span(0, 50, "SINGER"),), (Span(-1, 2, "SINGER"),),
                      (Span(0, 0, "SINGER"),), (Span(True, 2, "SINGER"),),
                      (Span(0, 2, ""),), (Span(0, 2, "A"), Span(1, 3, "B")),
                      (Span(0, 2, "A"), Span(0, 2, "A")), ((0, 2, "A"),)]:
            with self.subTest(spans=spans), self.assertRaises(ValueError):
                validate_spans("abc", spans)

    def test_unicode_offsets_are_python_codepoints(self):
        row = Record("unicode", "א😀ב", (Span(1, 2, "SINGER"),))
        self.assertEqual(evaluate([row], lambda _: [row.spans])["micro"]["tp"], 1)

    def test_artist_order_is_preserved(self):
        row = Record("duet", "א ב", (Span(0, 1, "SINGER"), Span(2, 3, "SINGER")))
        result = evaluate([row], lambda _: [(Span(2, 3, "SINGER"), Span(0, 1, "SINGER"))])
        self.assertEqual(result["exact_ordered_artist_records"], 1)

    def test_spacy_adapter_calls_pipe_once(self):
        class Entity:
            start_char, end_char, label_ = 0, 2, "SINGER"
        class Doc:
            ents = [Entity()]
        class Model:
            calls = 0
            def pipe(self, texts, *, batch_size):
                self.calls += 1
                self.batch_size = batch_size
                for _ in texts:
                    yield Doc()
        model = Model()
        result = evaluate([Record("a", "אב", (Span(0, 2, "SINGER"),))], spacy_predictor(model, batch_size=32))
        self.assertEqual(model.calls, 1)
        self.assertEqual(model.batch_size, 32)
        self.assertEqual(result["micro"]["f1"], 1)


class ControllerTests(unittest.TestCase):
    def test_raw_best_and_significant_anchor_are_separate(self):
        controller = DevelopmentStopper(StopConfig(min_delta=.125))
        controller.observe(.5, 1)
        slight = controller.observe(.5625, 2)
        self.assertTrue(slight.new_raw_best)
        self.assertEqual(slight.significant_anchor, .5)
        equal = controller.observe(.625, 3)
        self.assertEqual(equal.bad_checks, 2)
        self.assertTrue(equal.lr_reduced)
        accumulated = controller.observe(.6875, 4)
        self.assertEqual(accumulated.bad_checks, 0)
        self.assertEqual(accumulated.significant_anchor, .6875)

    def test_plateau_halves_lr_then_stops(self):
        controller = DevelopmentStopper(StopConfig())
        decisions = [controller.observe(.5, index) for index in range(1, 6)]
        self.assertFalse(decisions[1].lr_reduced)
        self.assertTrue(decisions[2].lr_reduced)
        self.assertEqual(decisions[2].lr_multiplier, .5)
        self.assertEqual(decisions[-1].reason, "DEV_PATIENCE")
        self.assertFalse(decisions[-1].lr_reduced)
        self.assertEqual(decisions[-1].best_raw_check, 1)

    def test_time_cap_is_inclusive_and_not_convergence(self):
        controller = DevelopmentStopper(StopConfig(time_cap_seconds=10))
        controller.observe(.5, 1)
        decision = controller.observe(.99, 10)
        self.assertEqual(decision.reason, "TIME_CAP")
        self.assertEqual(decision.best_raw_score, .5)
        self.assertEqual(decision.scored_checks, 1)

    def test_prework_time_guard(self):
        controller = DevelopmentStopper(StopConfig(time_cap_seconds=1))
        self.assertEqual(controller.check_time(1).reason, "TIME_CAP")
        with self.assertRaises(ValueError):
            controller.observe(.8, 2)

    def test_time_cannot_regress(self):
        controller = DevelopmentStopper(StopConfig())
        controller.check_time(2)
        with self.assertRaises(ValueError):
            controller.observe(.5, 1)

    def test_invalid_scores_rejected_without_state_change(self):
        controller = DevelopmentStopper(StopConfig())
        for value in (float("nan"), float("inf"), -1, 2, True):
            with self.subTest(value=value), self.assertRaises(ValueError):
                controller.observe(value, 0)
        self.assertEqual(controller.events, [])

    def test_invalid_configuration(self):
        for kwargs in ({"patience": 0}, {"patience": True}, {"min_delta": float("nan")},
                       {"time_cap_seconds": 0}, {"lr_factor": 1}, {"lr_patience": 1.2}):
            with self.subTest(kwargs=kwargs), self.assertRaises(ValueError):
                StopConfig(**kwargs)

    def test_snapshot_replay_matches_uninterrupted_control(self):
        first = DevelopmentStopper(StopConfig())
        first.check_time(0)
        first.observe(.5, 1)
        first.observe(.49, 2)
        restored = DevelopmentStopper.restore(json.loads(json.dumps(first.snapshot())))
        for score, elapsed in ((.49, 3), (.6, 4), (.6, 5)):
            self.assertEqual(first.observe(score, elapsed), restored.observe(score, elapsed))

    def test_snapshot_tampered_counter_rejected(self):
        controller = DevelopmentStopper(StopConfig())
        controller.observe(.5, 1)
        state = controller.snapshot()
        state["decision"]["bad_checks"] = 88
        with self.assertRaises(ValueError):
            DevelopmentStopper.restore(state)

    def test_snapshot_boolean_counter_does_not_equal_integer(self):
        controller = DevelopmentStopper(StopConfig())
        controller.observe(.5, 1)
        state = controller.snapshot()
        state["decision"]["best_raw_check"] = True
        with self.assertRaises(ValueError):
            DevelopmentStopper.restore(state)

    def test_per_batch_time_guards_do_not_accumulate_history(self):
        controller = DevelopmentStopper(StopConfig(time_cap_seconds=2000))
        for elapsed in range(1000):
            controller.check_time(elapsed)
        self.assertEqual(len(controller.events), 0)
        restored = DevelopmentStopper.restore(controller.snapshot())
        self.assertEqual(restored.elapsed, 999)
        with self.assertRaises(ValueError):
            restored.observe(.5, 998)

    def test_terminal_state_cannot_be_revived_by_restore(self):
        controller = DevelopmentStopper(StopConfig(time_cap_seconds=1))
        controller.check_time(1)
        restored = DevelopmentStopper.restore(controller.snapshot())
        with self.assertRaises(ValueError):
            restored.observe(.9, 2)


class SplitTests(unittest.TestCase):
    def rows(self):
        return [Record(str(i), f"owned synthetic title {i}", (), (f"artist:{i}",)) for i in range(80)]

    def build(self, rows=None, **kwargs):
        return build_development_split(self.rows() if rows is None else rows,
            dataset_id="owned-synthetic-v1", source_reference="owned test fixture", seed="42", **kwargs)

    def test_order_independent_and_no_test_split(self):
        rows = self.rows()
        a, b = self.build(rows), self.build(list(reversed(rows)))
        self.assertEqual(a, b)
        self.assertEqual(set(a["membership"]), {"train", "development"})
        self.assertFalse(a["historical_continuation"])
        self.assertTrue(all(a["actual_counts"].values()))
        validate_manifest(a, rows)

    def test_transitive_shared_identity_group(self):
        rows = [Record("1", "one", (), ("artist:A",)),
                Record("2", "two", (), ("artist:A", "template:B")),
                Record("3", "three", (), ("template:B",))]
        manifest = self.build(rows)
        self.assertEqual(len(manifest["groups"]), 1)
        self.assertEqual(manifest["groups"][0]["record_ids"], ["1", "2", "3"])
        validate_manifest(manifest, rows)

    def test_normalized_text_keeps_duplicates_together_without_rewriting(self):
        rows = [Record("1", "AA  BB", ()), Record("2", "aa bb", ())]
        manifest = self.build(rows)
        self.assertEqual(len(manifest["groups"]), 1)
        self.assertEqual(rows[0].text, "AA  BB")
        self.assertEqual(normalized_text_key("é"), normalized_text_key("e\u0301"))

    def test_conflicting_exact_annotation_rejected(self):
        with self.assertRaises(ValueError):
            self.build([Record("1", "same", ()), Record("2", "same", (Span(0, 4, "SINGER"),))])

    def test_exclusions_use_hashes_without_sealed_records(self):
        row = Record("1", "owned text", (), ("artist:A",))
        for kwargs in ({"forbidden_record_ids": ["1"]},
                       {"forbidden_text_hashes": [normalized_text_key(row.text)]},
                       {"forbidden_group_hashes": [group_key_hash("artist:A")]}):
            with self.subTest(kwargs=kwargs), self.assertRaises(ValueError):
                self.build([row], **kwargs)

    def test_invalid_exclusion_hash_rejected(self):
        with self.assertRaises(ValueError):
            self.build(forbidden_text_hashes=["not-a-hash"])

    def test_manifest_content_change_rejected(self):
        rows = self.rows()
        manifest = self.build(rows)
        rows[0] = Record("0", "changed", (), ("artist:0",))
        with self.assertRaises(ValueError):
            validate_manifest(manifest, rows)

    def test_manifest_digest_change_rejected(self):
        manifest = self.build()
        manifest["actual_counts"]["train"] += 1
        with self.assertRaises(ValueError):
            validate_manifest(manifest, self.rows())

    def test_unicode_version_change_rejected(self):
        manifest = self.build()
        manifest["unicode_data_version"] = "unrecognized-version"
        manifest["manifest_sha256"] = digest({k: v for k, v in manifest.items() if k != "manifest_sha256"})
        with self.assertRaises(ValueError):
            validate_manifest(manifest, self.rows())

    def test_modified_assignment_even_with_recomputed_digest_rejected(self):
        rows = self.rows()
        manifest = self.build(rows)
        manifest["membership"]["development"].append(manifest["membership"]["train"].pop())
        manifest["actual_counts"] = {k: len(v) for k, v in manifest["membership"].items()}
        manifest["manifest_sha256"] = digest({k: v for k, v in manifest.items() if k != "manifest_sha256"})
        with self.assertRaises(ValueError):
            validate_manifest(manifest, rows)

    def test_no_leakage_checks_all_three_known_channels(self):
        for row in (Record("1", "different", ()), Record("2", "SAME", ()),
                    Record("3", "different", (), ("artist:A",))):
            with self.subTest(row=row), self.assertRaises(ValueError):
                assert_no_leakage([Record("1", "same", (), ("artist:A",))], [row])

    def test_undisclosed_identity_is_not_claimed_isolated(self):
        manifest = self.build([Record("1", "one", ()), Record("2", "two", ())])
        self.assertIn("unknown exposure is not ruled out", manifest["scope"])

    def test_bad_fractions_fail(self):
        for fraction in (0, 1, float("nan"), True):
            with self.subTest(fraction=fraction), self.assertRaises(ValueError):
                self.build(development_fraction=fraction)


class ReceiptTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.evidence = {}
        for name in ("dataset_manifest", "training_config", "code_receipt", "campaign_ledger"):
            path = self.root / (name + ".json")
            path.write_text(json.dumps({"owned_synthetic_test": True, "name": name}))
            self.evidence[name] = path
        self.run = self.root / "run"

    def create(self):
        return create_run(self.run, run_id="owned-synthetic-run", purpose="controller unit tests only", evidence_paths=self.evidence)

    def test_chain_recoverable_without_modifying_campaign_evidence(self):
        before = self.evidence["campaign_ledger"].read_bytes()
        first = self.create()
        second = append_receipt(self.run, "metric", {"f0_5": .5}, expected_head_sha256=first["sha256"])
        state = inspect_run(self.run, expected_head_sha256=second["sha256"])
        self.assertEqual(state["count"], 2)
        self.assertFalse(state["automatic_training_resume_authorized"])
        self.assertFalse(state["interrupted_writer"])
        self.assertEqual(self.evidence["campaign_ledger"].read_bytes(), before)
        self.assertNotIn("owned_synthetic_test", (self.run / "00000000.json").read_text())

    def test_missing_campaign_input_blocks_before_creating_run(self):
        del self.evidence["campaign_ledger"]
        with self.assertRaises(ValueError):
            self.create()
        self.assertFalse(self.run.exists())

    def test_missing_evidence_bytes_fail(self):
        self.evidence["campaign_ledger"].unlink()
        with self.assertRaises(ValueError):
            self.create()
        self.assertFalse(self.run.exists())

    def test_run_directory_cannot_overwrite_old_run(self):
        self.create()
        with self.assertRaises(FileExistsError):
            self.create()

    def test_stale_writer_is_not_auto_removed(self):
        first = self.create()
        lock = self.run / ".writer.lock"
        lock.write_text("unknown-owner")
        with self.assertRaises(ValueError):
            append_receipt(self.run, "metric", {}, expected_head_sha256=first["sha256"])
        self.assertEqual(lock.read_text(), "unknown-owner")
        self.assertTrue(inspect_run(self.run)["interrupted_writer"])

    def test_corrupted_receipt_rejected(self):
        self.create()
        path = self.run / "00000000.json"
        value = json.loads(path.read_bytes())
        value["payload"]["details"]["run_id"] = "changed"
        path.write_bytes(canonical_bytes(value))
        with self.assertRaises(ValueError):
            inspect_run(self.run)

    def test_truncation_detected_with_retained_head(self):
        first = self.create()
        second = append_receipt(self.run, "metric", {}, expected_head_sha256=first["sha256"])
        (self.run / "00000001.json").unlink()
        with self.assertRaises(ValueError):
            inspect_run(self.run, expected_head_sha256=second["sha256"])

    def test_stale_expected_head_cannot_append(self):
        first = self.create()
        append_receipt(self.run, "metric", {}, expected_head_sha256=first["sha256"])
        with self.assertRaises(ValueError):
            append_receipt(self.run, "metric", {}, expected_head_sha256=first["sha256"])
        self.assertEqual(inspect_run(self.run)["count"], 2)

    def test_failed_atomic_publish_leaves_visible_pending_receipt(self):
        first = self.create()
        with patch("receipts.os.link", side_effect=OSError("synthetic unsupported hard links")):
            with self.assertRaises(OSError):
                append_receipt(self.run, "metric", {}, expected_head_sha256=first["sha256"])
        state = inspect_run(self.run)
        self.assertEqual(state["count"], 1)
        self.assertTrue(state["interrupted_writer"])
        with self.assertRaises(ValueError):
            append_receipt(self.run, "metric", {}, expected_head_sha256=first["sha256"])

    def test_oversized_or_nonfinite_receipt_rejected(self):
        first = self.create()
        for details in ({"raw": "x" * 70000}, {"score": float("nan")}):
            with self.subTest(details_size=len(str(details))), self.assertRaises(ValueError):
                append_receipt(self.run, "metric", details, expected_head_sha256=first["sha256"])
        self.assertEqual(inspect_run(self.run)["count"], 1)
        self.assertFalse(inspect_run(self.run)["interrupted_writer"])


if __name__ == "__main__":
    unittest.main()
