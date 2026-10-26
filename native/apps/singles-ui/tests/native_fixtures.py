"""Exact bytes for owned native acceptance fixtures, independent of OS newlines."""


def write_import_csv(path):
    # write_text translates LF on Windows, turning explicit CRLF into CRCRLF.
    # Keep the two real CRLF records intact; do not change CSV parser semantics.
    path.write_bytes('אמן מיובא,תיקייה מיובאת\r\nאמן מיובא,תיקייה שנייה\r\n'.encode('utf-8'))
