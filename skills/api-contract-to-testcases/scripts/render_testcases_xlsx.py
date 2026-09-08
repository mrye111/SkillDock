#!/usr/bin/env python3
import json
import sys
from pathlib import Path

from contract_skill_lib import write_cases_xlsx

sys.stdout.reconfigure(encoding="utf-8")


def load_rows(source: Path) -> list[dict]:
    payload = json.loads(source.read_text(encoding="utf-8-sig"))
    if isinstance(payload, dict):
        return payload.get("test_cases", [])
    if isinstance(payload, list):
        return payload
    raise ValueError("输入 JSON 必须是列表，或包含 test_cases 字段的对象。")


def main() -> None:
    if len(sys.argv) != 3:
        raise SystemExit("用法: render_testcases_xlsx.py <cases.json> <output.xlsx>")

    rows = load_rows(Path(sys.argv[1]))
    output = Path(sys.argv[2])
    write_cases_xlsx(rows, output)
    print(str(output))


if __name__ == "__main__":
    main()
