#!/usr/bin/env python3
import json
import sys
from pathlib import Path

from contract_skill_lib import build_review_report

sys.stdout.reconfigure(encoding="utf-8")


def load_json(path: Path):
    return json.loads(path.read_text(encoding="utf-8-sig"))


def main() -> None:
    if len(sys.argv) != 3:
        raise SystemExit("用法: review_testcases.py <normalized_contract.json> <cases.json>")

    contract = load_json(Path(sys.argv[1]))
    cases_data = load_json(Path(sys.argv[2]))
    cases = cases_data.get("test_cases", cases_data if isinstance(cases_data, list) else [])
    report = build_review_report(contract, cases)
    print(json.dumps(report, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
