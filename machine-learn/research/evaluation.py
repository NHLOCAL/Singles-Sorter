"""Single-pass strict character-span development evaluation (stdlib only).

This module does not load corpora/models, choose a dataset, or authorize use of
a sealed split. A caller must pass its explicitly admitted development records.
"""
from dataclasses import dataclass
from itertools import zip_longest
from typing import Callable, Iterable, Sequence


@dataclass(frozen=True, order=True)
class Span:
    start: int
    end: int
    label: str


@dataclass(frozen=True)
class Record:
    record_id: str
    text: str
    spans: tuple[Span, ...]
    group_keys: tuple[str, ...] = ()


def validate_spans(text: str, spans: Sequence[Span]) -> tuple[Span, ...]:
    """Reject invalid/overlapping spans, never silently realign or skip gold."""
    if not isinstance(text, str):
        raise ValueError("text must be a string")
    if any(not isinstance(span, Span) or type(span.start) is not int
           or type(span.end) is not int or not isinstance(span.label, str) for span in spans):
        raise ValueError("spans must contain typed character offsets and labels")
    ordered = tuple(sorted(spans))
    previous_end = 0
    for span in ordered:
        if (not isinstance(span, Span) or type(span.start) is not int
                or type(span.end) is not int or not isinstance(span.label, str)
                or not span.label or not 0 <= span.start < span.end <= len(text)
                or span.start < previous_end):
            raise ValueError("invalid, duplicate, or overlapping character span")
        previous_end = span.end
    return ordered


def spacy_predictor(nlp, *, batch_size: int = 64):
    """Adapter for an already authenticated spaCy model; does not load one."""
    if type(batch_size) is not int or batch_size < 1:
        raise ValueError("batch size must be a positive integer")

    def predict(texts):
        for doc in nlp.pipe(texts, batch_size=batch_size):
            yield tuple(Span(ent.start_char, ent.end_char, ent.label_) for ent in doc.ents)

    return predict


def validate_records(records: Sequence[Record]) -> None:
    seen = set()
    for record in records:
        if (not isinstance(record, Record) or not isinstance(record.record_id, str)
                or not record.record_id or record.record_id in seen):
            raise ValueError("record IDs must be nonempty and unique")
        seen.add(record.record_id)
        validate_spans(record.text, record.spans)
        if (any(not isinstance(k, str) or not k for k in record.group_keys)
                or len(set(record.group_keys)) != len(record.group_keys)):
            raise ValueError("group keys must be nonempty unique strings")


def scores(tp: int, fp: int, fn: int) -> dict:
    if any(type(n) is not int or n < 0 for n in (tp, fp, fn)):
        raise ValueError("counts must be nonnegative integers")
    precision = tp / (tp + fp) if tp + fp else 0.0
    recall = tp / (tp + fn) if tp + fn else 0.0
    return {
        "tp": tp, "fp": fp, "fn": fn,
        "precision": precision, "recall": recall,
        "f1": 2 * tp / (2 * tp + fp + fn) if 2 * tp + fp + fn else 0.0,
        "f0_5": 1.25 * tp / (1.25 * tp + fp + .25 * fn)
        if 1.25 * tp + fp + .25 * fn else 0.0,
    }


def evaluate(
    records: Sequence[Record],
    predict_many: Callable[[Iterable[str]], Iterable[Sequence[Span]]],
    *,
    labels: Sequence[str] = (),
    singer_label: str = "SINGER",
) -> dict:
    """Call predict_many once; reuse each prediction for every label and metric.

    A spaCy adapter can yield tuples from nlp.pipe(texts). The adapter must
    preserve offsets/tokenization; this evaluator never repairs annotations.
    The result deliberately contains no raw source texts or error examples.
    Supplementary-classifier comparisons should be a separate named prediction
    arm; this evaluator never promotes that classifier into a primary detector.
    """
    validate_records(records)
    if (not isinstance(singer_label, str) or not singer_label
            or any(not isinstance(x, str) or not x for x in labels)):
        raise ValueError("labels must be nonempty strings")
    counts = {label: [0, 0, 0] for label in set(labels) | {singer_label}}
    exact_records = exact_artist_lists = negative_records = false_artist_records = 0
    marker = object()
    predictions = predict_many(record.text for record in records)
    for record, predicted in zip_longest(records, predictions, fillvalue=marker):
        if record is marker or predicted is marker:
            raise ValueError("prediction count differs from record count")
        gold = validate_spans(record.text, record.spans)
        prediction = validate_spans(record.text, tuple(predicted))
        gold_set, pred_set = set(gold), set(prediction)
        exact_records += gold_set == pred_set
        for label in {s.label for s in gold_set | pred_set}:
            values = counts.setdefault(label, [0, 0, 0])
            g = {s for s in gold_set if s.label == label}
            p = {s for s in pred_set if s.label == label}
            values[0] += len(g & p)
            values[1] += len(p - g)
            values[2] += len(g - p)
        gold_artists = [record.text[s.start:s.end] for s in gold if s.label == singer_label]
        pred_artists = [record.text[s.start:s.end] for s in prediction if s.label == singer_label]
        exact_artist_lists += gold_artists == pred_artists
        if not gold_artists:
            negative_records += 1
            false_artist_records += bool(pred_artists)
    total = len(records)
    summed = [sum(c[i] for c in counts.values()) for i in range(3)]
    return {
        "schema": "strict-character-span-evaluation-v1",
        "records": total,
        "per_label": {key: scores(*counts[key]) for key in sorted(counts)},
        "micro": scores(*summed),
        "exact_span_records": exact_records,
        "exact_ordered_artist_records": exact_artist_lists,
        "ordered_artist_accuracy": exact_artist_lists / total if total else 0.0,
        "negative_singer_records": negative_records,
        "false_singer_negative_records": false_artist_records,
        "negative_singer_false_positive_rate": false_artist_records / negative_records
        if negative_records else 0.0,
        "misaligned_gold_skipped": 0,
        "prediction_passes": 1,
    }
