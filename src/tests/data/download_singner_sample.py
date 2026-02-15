"""Download a deterministic 5,000-row sample from NHLOCAL/SingNER."""

from __future__ import annotations

import json
from pathlib import Path

from datasets import load_dataset


OUTPUT_PATH = Path(__file__).with_name("singner_sample_5000.jsonl")
SAMPLE_SIZE = 5000


def main() -> None:
    dataset = load_dataset("NHLOCAL/SingNER", split="train")
    sample = dataset.select(range(SAMPLE_SIZE))

    with OUTPUT_PATH.open("w", encoding="utf-8") as output_file:
        for row in sample:
            output_file.write(
                json.dumps(
                    {"text": row["text"], "entities": row["entities"]},
                    ensure_ascii=False,
                )
                + "\n"
            )

    print(f"Saved {len(sample)} rows to {OUTPUT_PATH}")


if __name__ == "__main__":
    main()
