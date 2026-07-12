"""Pure validation for sorting requests."""

from __future__ import annotations

import os
from pathlib import Path

from .state import SortJob


def validate_job(job: SortJob) -> tuple[str, ...]:
    errors: list[str] = []
    source = _validate_folder(
        job.source, "יש לבחור תיקיית מוזיקה.", "תיקיית המוזיקה לא נמצאה.", errors
    )
    target = _validate_folder(job.target, "יש לבחור תיקיית יעד.", "תיקיית היעד לא נמצאה.", errors)

    if source and target and source == target:
        errors.append("תיקיית המוזיקה ותיקיית היעד חייבות להיות שונות.")
    elif source and target and target.is_relative_to(source):
        errors.append("תיקיית היעד אינה יכולה להיות בתוך תיקיית המוזיקה.")
    elif source and target and source.is_relative_to(target):
        errors.append("תיקיית המוזיקה אינה יכולה להיות בתוך תיקיית היעד.")
    if source:
        try:
            if not any(source.iterdir()):
                errors.append("תיקיית המוזיקה ריקה.")
        except OSError:
            errors.append("אין הרשאה לקרוא את תיקיית המוזיקה.")
    if target and target.exists() and not os.access(target, os.W_OK):
        errors.append("אין הרשאת כתיבה לתיקיית היעד.")
    return tuple(errors)


def _validate_folder(
    value: Path | None,
    missing_message: str,
    absent_message: str,
    errors: list[str],
) -> Path | None:
    if value is None:
        errors.append(missing_message)
        return None
    path = value.expanduser().resolve()
    if not path.is_dir():
        errors.append(absent_message)
        return None
    return path
