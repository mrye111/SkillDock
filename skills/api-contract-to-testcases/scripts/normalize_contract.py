#!/usr/bin/env python3
import json
import sys
from pathlib import Path

from contract_skill_lib import normalize_contract

sys.stdout.reconfigure(encoding="utf-8")


def main() -> None:
    if len(sys.argv) != 2:
        raise SystemExit("用法: normalize_contract.py <contract-file>")

    result = normalize_contract(Path(sys.argv[1]))
    print(json.dumps(result, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
