"""Independent reference tests: never import the replacement Rust policy."""
import os
from pathlib import Path
import sys

sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).parent))
from export_full_contract import verify_baseline

BASELINE = Path(os.environ["SINGLES_RELEASE_BASELINE"])
verify_baseline(BASELINE)
sys.path.insert(0, str(BASELINE / "src/core"))
