#!/usr/bin/env python3
import argparse
import json
import sys
from pathlib import Path

from contract_skill_lib import (
    build_review_report,
    filter_contract,
    generate_testcase_payload,
    grouped_contracts_by_tag,
    normalize_contract,
    public_prerequisite_paths,
    render_pending_markdown,
    render_review_markdown,
    render_test_points_markdown,
    slugify_text,
    write_cases_xlsx,
)

sys.stdout.reconfigure(encoding="utf-8")


def stem_name(path: Path) -> str:
    return path.stem


def write_json(path: Path, payload: dict) -> None:
    path.write_text(json.dumps(payload, ensure_ascii=False, indent=2), encoding="utf-8")


def verify_prerequisite_integrity(contract: dict) -> None:
    endpoints = contract["normalized"].get("endpoints", [])
    available_paths = {endpoint.get("path") for endpoint in endpoints}
    missing: dict[str, list[str]] = {}

    for endpoint in endpoints:
        required_paths = [path for path in public_prerequisite_paths(endpoint) if path not in available_paths]
        if required_paths:
            missing[f"{endpoint['method']} {endpoint['path']}"] = required_paths

    if missing:
        raise SystemExit(
            "生成结果缺少公共前置接口，无法继续输出完整用例: "
            + json.dumps(missing, ensure_ascii=False)
        )


def write_bundle(contract: dict, output_dir: Path, base_name: str, keep_json: bool) -> dict:
    verify_prerequisite_integrity(contract)
    payload = generate_testcase_payload(contract)
    review_report = build_review_report(contract, payload["test_cases"])

    normalized_json = output_dir / f"{base_name}_normalized_contract.json"
    cases_json = output_dir / f"{base_name}_testcases.json"
    if keep_json:
        write_json(normalized_json, contract)
        write_json(cases_json, payload)

    point_file = output_dir / f"{base_name}_接口测试要点.md"
    case_file = output_dir / f"{base_name}_接口测试用例.xlsx"
    pending_file = output_dir / f"{base_name}_待确认项.md"
    review_file = output_dir / f"{base_name}_审核评分报告.md"

    point_file.write_text(render_test_points_markdown(contract, payload), encoding="utf-8")
    write_cases_xlsx(payload["test_cases"], case_file)
    pending_file.write_text(render_pending_markdown(payload), encoding="utf-8")
    review_file.write_text(render_review_markdown(review_report), encoding="utf-8")

    files = [str(point_file), str(case_file), str(pending_file), str(review_file)]
    if keep_json:
        files.extend([str(normalized_json), str(cases_json)])
    auto_included = contract["normalized"].get("auto_included_endpoints", [])
    return {
        "output_dir": str(output_dir),
        "files": files,
        "endpoint_count": len(contract["normalized"].get("endpoints", [])),
        "case_count": len(payload["test_cases"]),
        "pending_count": len(payload["pending_items"]),
        "auto_included_endpoints": auto_included,
        "review_score": review_report["score"],
        "review_conclusion": review_report["conclusion"],
    }


def main() -> None:
    parser = argparse.ArgumentParser(description="根据接口契约一键生成接口测试交付物。")
    parser.add_argument("contract_file", help="Swagger/OpenAPI/纯文本契约文件路径")
    parser.add_argument("--output-dir", dest="output_dir", help="输出目录，默认使用契约文件同级目录下的同名文件夹")
    parser.add_argument("--keep-json", action="store_true", help="保留中间 JSON 文件")
    parser.add_argument("--tag", action="append", default=[], help="只生成指定 tag/模块，支持多次传入")
    parser.add_argument("--interface-name", action="append", default=[], help="只生成接口中文名称包含指定关键字的接口，支持多次传入")
    parser.add_argument("--path-keyword", action="append", default=[], help="只生成 path 包含指定关键字的接口，支持多次传入")
    parser.add_argument("--split-by-tag", action="store_true", help="按 tag 拆分输出，每个 tag 生成一套独立交付物")
    args = parser.parse_args()

    contract_path = Path(args.contract_file)
    base_name = stem_name(contract_path)
    output_dir = Path(args.output_dir) if args.output_dir else contract_path.with_name(base_name)
    output_dir.mkdir(parents=True, exist_ok=True)

    contract = normalize_contract(contract_path)
    filtered_contract = filter_contract(
        contract,
        tag_filters=args.tag,
        interface_name_filters=args.interface_name,
        path_keywords=args.path_keyword,
    )

    if not filtered_contract["normalized"].get("endpoints"):
        raise SystemExit("筛选后没有匹配到任何接口，请调整 --tag / --interface-name / --path-keyword 条件。")

    if args.split_by_tag:
        bundles = []
        split_root = output_dir / "by_tag"
        split_root.mkdir(parents=True, exist_ok=True)
        for tag_name, grouped_contract in grouped_contracts_by_tag(filtered_contract, source_contract=contract):
            tag_slug = slugify_text(tag_name)
            tag_dir = split_root / tag_slug
            tag_dir.mkdir(parents=True, exist_ok=True)
            bundles.append({
                "tag": tag_name,
                **write_bundle(grouped_contract, tag_dir, f"{base_name}_{tag_slug}", args.keep_json),
            })
        summary = {
            "mode": "split_by_tag",
            "filter": {
                "tag": args.tag,
                "interface_name": args.interface_name,
                "path_keyword": args.path_keyword,
            },
            "bundle_count": len(bundles),
            "bundles": bundles,
        }
    else:
        summary = {
            "mode": "single_bundle",
            "filter": {
                "tag": args.tag,
                "interface_name": args.interface_name,
                "path_keyword": args.path_keyword,
            },
            **write_bundle(filtered_contract, output_dir, base_name, args.keep_json),
        }

    print(json.dumps(summary, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
