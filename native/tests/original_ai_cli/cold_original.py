"""Run the exact frozen original CLI once in a fresh owned Windows process."""
from pathlib import Path
import os
import runpy
import sys
from windows_v14_trace import confine_mutations


def main():
    if os.name != 'nt' or os.environ.get('GITHUB_ACTIONS') != 'true' or not __debug__:
        raise RuntimeError('Cold original trial requires the owned Windows CI runner and assertions')
    root, script = Path(sys.argv[1]).resolve(strict=True), Path(sys.argv[2]).resolve(strict=True)
    if Path.cwd().resolve(strict=True) != root or root.parent != script.parent.parent:
        raise RuntimeError('Cold original paths must belong to one exact authored trial')
    sys.path.insert(0, str(script.parent))
    sys.argv = [str(script), 'source', 'target', '-c', '-d', '-n', '-l', 'DEBUG']
    # Source code/model hashes were verified before process launch. All reachable
    # original filesystem mutation remains confined to this fresh authored tree.
    with confine_mutations(root):
        runpy.run_path(str(script), run_name='__main__')


if __name__ == '__main__': main()
