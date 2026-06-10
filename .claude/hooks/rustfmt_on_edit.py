"""PostToolUse hook: format an edited Rust file with rustfmt.

Reads the hook JSON payload from stdin, extracts the edited file path, and
runs `rustfmt` on it if it is a .rs file. Failures are swallowed so the hook
never blocks an edit.
"""
import json
import subprocess
import sys


def main() -> int:
    try:
        data = json.load(sys.stdin)
    except Exception:
        return 0
    tool_input = data.get("tool_input") or {}
    path = tool_input.get("file_path") or ""
    if not path.endswith(".rs"):
        return 0
    try:
        subprocess.run(
            ["rustfmt", "--edition", "2021", path],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
    except Exception:
        pass
    return 0


if __name__ == "__main__":
    sys.exit(main())
