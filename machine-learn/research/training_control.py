"""Pure development-score stopping policy; not a training/budget supervisor.

Elapsed time must include fitting, evaluation, checkpointing and shutdown work.
Call check_time before every unit of work. This cooperative helper cannot kill
an unresponsive process or authorize spending an existing campaign budget.
"""
from dataclasses import asdict, dataclass
import json
import math


@dataclass(frozen=True)
class StopConfig:
    min_delta: float = 0.001
    patience: int = 4
    lr_patience: int = 2
    lr_factor: float = 0.5
    time_cap_seconds: float = 1800.0

    def __post_init__(self):
        for name in ("min_delta", "lr_factor", "time_cap_seconds"):
            value = getattr(self, name)
            if type(value) not in (int, float) or not math.isfinite(value):
                raise ValueError(f"{name} must be finite")
        if not 0 <= self.min_delta < 1 or not 0 < self.lr_factor < 1:
            raise ValueError("invalid min_delta or lr_factor")
        if self.time_cap_seconds <= 0:
            raise ValueError("time cap must be positive")
        if any(type(n) is not int or n < 1 for n in (self.patience, self.lr_patience)):
            raise ValueError("patience values count positive integer scored checks")


@dataclass(frozen=True)
class Decision:
    stopped: bool
    reason: str | None
    scored_checks: int
    best_raw_score: float | None
    best_raw_check: int | None
    significant_anchor: float | None
    bad_checks: int
    lr_multiplier: float
    new_raw_best: bool = False
    lr_reduced: bool = False


class DevelopmentStopper:
    """Maximize fixed development F0.5; tiny gains accumulate against an anchor.

    Every raw best is retained separately even when below min_delta. A score
    exactly min_delta above the anchor is not a significant improvement. The
    first valid scored check establishes the anchor. LR reduces on each full
    lr_patience interval, but no new LR change is emitted at terminal stopping.
    A resource stop is never labeled convergence. Scores completed at/after the
    inclusive time cap do not become selected checkpoints.
    """
    def __init__(self, config: StopConfig):
        if not isinstance(config, StopConfig):
            raise ValueError("an explicit validated StopConfig is required")
        self.config = config
        self.events: list[dict] = []
        self.elapsed = 0.0
        self.best_raw = self.anchor = None
        self.best_check = None
        self.checks = self.bad = self.reductions = 0
        self.reason = None

    def _validate_elapsed(self, elapsed):
        if (type(elapsed) not in (int, float) or not math.isfinite(elapsed)
                or elapsed < self.elapsed):
            raise ValueError("elapsed time must be finite, nonnegative and monotonic")

    def _decision(self, *, new_raw_best=False, lr_reduced=False):
        return Decision(self.reason is not None, self.reason, self.checks,
                        self.best_raw, self.best_check, self.anchor, self.bad,
                        self.config.lr_factor ** self.reductions,
                        new_raw_best, lr_reduced)

    def check_time(self, elapsed_seconds: float) -> Decision:
        self._validate_elapsed(elapsed_seconds)
        if self.reason is not None:
            raise ValueError("controller is terminal; a new run must not pretend to resume it")
        self.elapsed = elapsed_seconds
        if elapsed_seconds >= self.config.time_cap_seconds:
            self.events.append({"kind": "time", "elapsed_seconds": elapsed_seconds})
            self.reason = "TIME_CAP"
        return self._decision()

    def observe(self, development_f0_5: float, elapsed_seconds: float) -> Decision:
        if (type(development_f0_5) not in (int, float)
                or not math.isfinite(development_f0_5)
                or not 0 <= development_f0_5 <= 1):
            raise ValueError("development F0.5 must be finite and in [0, 1]")
        self._validate_elapsed(elapsed_seconds)
        if self.reason is not None:
            raise ValueError("controller is terminal")
        self.elapsed = elapsed_seconds
        self.events.append({"kind": "score", "score": development_f0_5,
                            "elapsed_seconds": elapsed_seconds})
        if elapsed_seconds >= self.config.time_cap_seconds:
            self.reason = "TIME_CAP"
            return self._decision()
        self.checks += 1
        new_best = self.best_raw is None or development_f0_5 > self.best_raw
        if new_best:
            self.best_raw, self.best_check = development_f0_5, self.checks
        if self.anchor is None or development_f0_5 > self.anchor + self.config.min_delta:
            self.anchor, self.bad = development_f0_5, 0
        else:
            self.bad += 1
        if self.bad >= self.config.patience:
            self.reason = "DEV_PATIENCE"
            return self._decision(new_raw_best=new_best)
        reduce_lr = self.bad > 0 and self.bad % self.config.lr_patience == 0
        if reduce_lr:
            self.reductions += 1
        return self._decision(new_raw_best=new_best, lr_reduced=reduce_lr)

    def snapshot(self) -> dict:
        return {"schema": "development-stopper-v1", "config": asdict(self.config),
                "events": [dict(event) for event in self.events],
                "elapsed_seconds": self.elapsed,
                "decision": asdict(self._decision())}

    @classmethod
    def restore(cls, snapshot: dict):
        """Replay validated event history rather than trusting derived counters.

        Controller recovery alone does not restore model/optimizer/RNG state or
        prove ownership/accounting. External receipts must bind those artifacts.
        """
        if (type(snapshot) is not dict
                or set(snapshot) != {"schema", "config", "events", "elapsed_seconds", "decision"}
                or snapshot["schema"] != "development-stopper-v1"
                or type(snapshot["events"]) is not list):
            raise ValueError("invalid controller snapshot")
        try:
            controller = cls(StopConfig(**snapshot["config"]))
            for event in snapshot["events"]:
                if event.get("kind") == "time" and set(event) == {"kind", "elapsed_seconds"}:
                    controller.check_time(event["elapsed_seconds"])
                elif event.get("kind") == "score" and set(event) == {"kind", "score", "elapsed_seconds"}:
                    controller.observe(event["score"], event["elapsed_seconds"])
                else:
                    raise ValueError("invalid controller event")
            if controller.reason is None:
                controller.check_time(snapshot["elapsed_seconds"])
            elif (type(snapshot["elapsed_seconds"]) not in (int, float)
                  or snapshot["elapsed_seconds"] != controller.elapsed):
                raise ValueError("terminal elapsed time differs from its event")
        except (TypeError, AttributeError, KeyError) as exc:
            raise ValueError("invalid controller snapshot") from exc
        if json.dumps(asdict(controller._decision()), sort_keys=True, allow_nan=False) != json.dumps(snapshot["decision"], sort_keys=True, allow_nan=False):
            raise ValueError("controller history and derived state disagree")
        return controller
