"""Exercise the actual dump helper without importing heavyweight oracle packages."""
import ast
import hashlib
import io
import json
from pathlib import Path
import tempfile
import unittest

HERE = Path(__file__).resolve().parent


def dump_helper():
    source = ast.parse((HERE / "released-ner-export/export.py").read_text(encoding="utf-8"))
    function = next(node for node in source.body if isinstance(node, ast.FunctionDef) and node.name == "dump")
    namespace = {"json": json}
    exec(compile(ast.Module(body=[function], type_ignores=[]), "export.py:dump", "exec"), namespace)
    return namespace["dump"]


class WindowsTextPath:
    """Emulate Python's native Windows newline translation without a Windows VM."""
    def __init__(self):
        self.data = b""

    def write_text(self, text, encoding):
        raw = io.BytesIO()
        writer = io.TextIOWrapper(raw, encoding=encoding, newline="\r\n")
        count = writer.write(text)
        writer.flush()
        self.data = raw.getvalue()
        return count

    def write_bytes(self, data):
        self.data = bytes(data)
        return len(data)


class ExportBytes(unittest.TestCase):
    def test_windows_export_keeps_exact_frozen_tokenizer_bytes(self):
        expected = (HERE / "released-ner-text/tokenizer.json").read_bytes()
        self.assertEqual(hashlib.sha256(expected).hexdigest(),
                         "33841cad4949620c48fbb0570865a31a19cfbb463088b1fec3d9fb7e1a8c8574")
        path = WindowsTextPath()
        dump_helper()(path, json.loads(expected))
        self.assertEqual(path.data, expected)

    def test_real_file_preserves_utf8_and_lf_without_changing_values(self):
        value = {"text": "שלום\nline", "array": [1, True, None]}
        expected = (json.dumps(value, ensure_ascii=False, indent=2, allow_nan=False) + "\n").encode("utf-8")
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "export.json"
            dump_helper()(path, value)
            self.assertEqual(path.read_bytes(), expected)
            self.assertEqual(json.loads(path.read_bytes()), value)

    def test_nonfinite_values_still_fail_without_writing(self):
        path = WindowsTextPath()
        with self.assertRaises(ValueError):
            dump_helper()(path, {"bad": float("nan")})
        self.assertEqual(path.data, b"")


if __name__ == "__main__":
    unittest.main()
