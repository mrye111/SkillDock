#!/usr/bin/env python3
from __future__ import annotations

import copy
import html
import json
import re
from collections import defaultdict
from pathlib import Path
from typing import Any
from urllib.parse import parse_qs, urlencode, urlparse

from openpyxl import Workbook
from openpyxl.styles import Alignment, Font

try:
    import yaml
except ImportError:  # pragma: no cover
    yaml = None


CASE_HEADERS = ["接口中文名称", "前置条件", "测试场景", "path", "headers", "body", "预期响应结果", "断言响应", "断言sql"]
RUBRIC_DIMENSIONS = {
    "契约一致性": 30,
    "参数覆盖完整性": 20,
    "场景覆盖完整性": 20,
    "流程/状态覆盖": 10,
    "断言可执行性": 15,
    "模板规范性": 5,
}
PUBLIC_KEYWORDS = ("登录", "注册", "验证码", "/login", "/register", "/verificationCode")
PUBLIC_PREREQUISITE_PATHS = ("/user/register", "/user/login", "/owner/login", "/verificationCode/message")
STATE_KEYWORDS = ("审核", "绑定", "入住", "发布", "上架", "下架", "状态", "/examine", "/bind", "/checkIn", "/publish")
SUCCESS_HINTS = ("成功", "通过", "已发送", "已登录", "已上传", "已发布", "已处理", "操作成功")
ID_FIELD_SUFFIXES = ("Id", "ID")
ASSERTION_PATTERNS = [
    re.compile(r"^http_status\s*(==|!=|>=|<=|>|<)\s*\d+$"),
    re.compile(r"^\$\.[\w\.\[\]]+\s*(==|!=|>=|<=|>|<)\s*(.+)$"),
    re.compile(r'^\$\.[\w\.\[\]]+\s+contains\s+".+"$'),
    re.compile(r"^\$\.[\w\.\[\]]+\s+contains\s+'.+'$"),
    re.compile(r"^\$\.[\w\.\[\]]+\s+size\s*(==|!=|>=|<=|>|<)\s*\d+$"),
]


def read_text(path: Path) -> str:
    return path.read_text(encoding="utf-8-sig")


def unique_list(items: list[Any]) -> list[Any]:
    seen = set()
    result = []
    for item in items:
        marker = json.dumps(item, ensure_ascii=False, sort_keys=True) if isinstance(item, (dict, list)) else str(item)
        if marker in seen:
            continue
        seen.add(marker)
        result.append(item)
    return result


def strip_html(raw: str) -> str:
    text = html.unescape(raw or "")
    replacements = {
        "</br>": "\n",
        "<br/>": "\n",
        "<br>": "\n",
        "</p>": "\n",
        "<p>": "",
        "</tr>": "\n",
        "<tr>": "",
        "</table>": "\n",
        "<table>": "\n",
        "</td>": "\t",
        "<td>": "",
        "</th>": "\t",
        "<th>": "",
        "&nbsp;": " ",
    }
    for source, target in replacements.items():
        text = text.replace(source, target)
    text = re.sub(r"<[^>]+>", "", text)
    text = re.sub(r"\r\n?", "\n", text)
    text = re.sub(r"\n{3,}", "\n\n", text)
    return "\n".join(line.strip() for line in text.splitlines()).strip()


def clean_summary(summary: str) -> str:
    text = re.sub(r"--【.*?】", "", summary or "")
    text = re.sub(r"\s+", " ", text)
    return text.strip() or "未命名接口"


def detect_format(text: str) -> str:
    stripped = text.lstrip()
    if stripped.startswith("{") or stripped.startswith("["):
        return "swagger-json"
    if ("openapi:" in text or "swagger:" in text) and "paths:" in text:
        return "swagger-yaml"
    return "plain-text"


def parse_contract(path: Path) -> tuple[str, dict | str]:
    text = read_text(path)
    fmt = detect_format(text)
    if fmt == "swagger-json":
        return fmt, json.loads(text)
    if fmt == "swagger-yaml":
        if yaml is None:
            raise SystemExit("当前环境缺少 PyYAML，无法解析 YAML 契约。")
        return fmt, yaml.safe_load(text)
    return fmt, text


def is_openapi3(data: dict) -> bool:
    return bool(data.get("openapi"))


def get_schema_components(data: dict) -> dict[str, dict]:
    if is_openapi3(data):
        return (data.get("components") or {}).get("schemas") or {}
    return data.get("definitions") or {}


def trim_url(url: str) -> str:
    return re.sub(r'[)>,"\']+$', "", url.strip())


def merge_object_schemas(base: dict, extra: dict) -> dict:
    result = copy.deepcopy(base)
    for key, value in extra.items():
        if key == "required":
            result[key] = unique_list(list(result.get(key, [])) + list(value or []))
        elif key == "properties" and isinstance(value, dict):
            props = result.setdefault("properties", {})
            props.update(value)
        elif key == "allOf":
            continue
        else:
            result[key] = value
    return result


def extract_constraints_from_description(description: str) -> dict[str, Any]:
    text = description or ""
    constraints: dict[str, Any] = {}

    range_match = re.search(r"长度(?:为|在)?\s*(\d+)\s*(?:到|~|-)\s*(\d+)", text)
    if range_match:
        constraints["minLength"] = int(range_match.group(1))
        constraints["maxLength"] = int(range_match.group(2))

    max_match = re.search(r"长度(?:不能)?大于\s*(\d+)", text)
    if max_match and "maxLength" not in constraints:
        constraints["maxLength"] = int(max_match.group(1))

    min_match = re.search(r"长度(?:不能)?小于\s*(\d+)", text)
    if min_match and "minLength" not in constraints:
        constraints["minLength"] = int(min_match.group(1))

    numeric_only = any(token in text for token in ("只能是数字", "仅支持数字", "数字字符串"))
    if numeric_only:
        constraints["pattern"] = r"^\d+$"
        constraints["x-numeric-only"] = True

    positive_number = re.search(r"大于\s*0\s*的数字", text)
    if positive_number:
        constraints["minimum"] = 0
        constraints["exclusiveMinimum"] = True

    minimum_match = re.search(r"最小(?:值)?(?:为)?\s*(\d+)", text)
    if minimum_match and "minimum" not in constraints:
        constraints["minimum"] = int(minimum_match.group(1))

    maximum_match = re.search(r"最大(?:值)?(?:为)?\s*(\d+)", text)
    if maximum_match and "maximum" not in constraints:
        constraints["maximum"] = int(maximum_match.group(1))

    if "手机号" in text:
        constraints.setdefault("pattern", r"^1\d{10}$")
        constraints.setdefault("x-format-hint", "phone")
    if "邮箱" in text:
        constraints.setdefault("format", "email")
    if "身份证" in text:
        constraints.setdefault("x-format-hint", "id-card")
    if "字母" in text and "数字" in text:
        constraints.setdefault("pattern", r"^(?=.*[A-Za-z])(?=.*\d).+$")
        constraints.setdefault("x-format-hint", "alnum-mixed")
    file_size_match = re.search(r"最大\s*(\d+)\s*KB", text, re.I)
    if file_size_match:
        constraints["x-file-max-kb"] = int(file_size_match.group(1))
    file_type_match = re.search(r"(jpg、png|jpg,png|jpg/png)", text, re.I)
    if file_type_match:
        constraints["x-file-exts"] = ["jpg", "png"]
    return constraints


def normalize_schema(schema: dict | None, components: dict[str, dict], seen: set[str] | None = None) -> dict | None:
    if not schema:
        return None

    seen = seen or set()
    source = copy.deepcopy(schema)

    if "$ref" in source:
        ref = source["$ref"]
        if ref in seen:
            return {"title": ref.split("/")[-1], "type": "object"}
        ref_name = ref.split("/")[-1]
        target = components.get(ref_name, {})
        resolved = normalize_schema(target, components, seen | {ref}) or {}
        resolved.setdefault("title", ref_name)
        source.pop("$ref", None)
        source = merge_object_schemas(resolved, source)

    if "allOf" in source:
        merged: dict[str, Any] = {}
        for item in source.get("allOf") or []:
            merged = merge_object_schemas(merged, normalize_schema(item, components, seen) or {})
        source.pop("allOf", None)
        source = merge_object_schemas(merged, source)

    if "properties" in source:
        source["type"] = source.get("type") or "object"
        normalized_properties = {}
        for name, prop in (source.get("properties") or {}).items():
            normalized_properties[name] = normalize_schema(prop, components, seen)
        source["properties"] = normalized_properties

    if "items" in source and isinstance(source["items"], dict):
        source["items"] = normalize_schema(source["items"], components, seen)
        source["type"] = source.get("type") or "array"

    if isinstance(source.get("additionalProperties"), dict):
        source["additionalProperties"] = normalize_schema(source["additionalProperties"], components, seen)

    source.setdefault("type", "object" if source.get("properties") else source.get("type"))
    derived = extract_constraints_from_description(str(source.get("description") or ""))
    for key, value in derived.items():
        source.setdefault(key, value)
    return source


def flatten_schema_fields(schema: dict | None, prefix: str = "", required: bool = False) -> list[dict]:
    if not schema:
        return []
    fields: list[dict] = []
    current_required = set(schema.get("required") or [])
    schema_type = schema.get("type")
    if schema_type == "object" and schema.get("properties"):
        for name, prop in schema["properties"].items():
            path = f"{prefix}.{name}" if prefix else name
            prop_required = name in current_required if prefix == "" else required and name in current_required
            prop_fields = flatten_schema_fields(prop, path, prop_required)
            if prop_fields:
                fields.extend(prop_fields)
            else:
                leaf = {
                    "path": path,
                    "name": name,
                    "required": prop_required,
                    "type": prop.get("type", "object"),
                    "description": prop.get("description", ""),
                    "example": prop.get("example"),
                    "format": prop.get("format"),
                    "enum": prop.get("enum"),
                    "minimum": prop.get("minimum"),
                    "maximum": prop.get("maximum"),
                    "exclusiveMinimum": prop.get("exclusiveMinimum"),
                    "exclusiveMaximum": prop.get("exclusiveMaximum"),
                    "minLength": prop.get("minLength"),
                    "maxLength": prop.get("maxLength"),
                    "pattern": prop.get("pattern"),
                    "x-numeric-only": prop.get("x-numeric-only", False),
                    "x-format-hint": prop.get("x-format-hint"),
                }
                fields.append(leaf)
        return fields

    if schema_type == "array":
        items = schema.get("items") or {}
        if items.get("properties") or items.get("type") in {"object", "array"}:
            return flatten_schema_fields(items, f"{prefix}[0]" if prefix else "[0]", required)
        return [{
            "path": prefix or "[0]",
            "name": prefix.split(".")[-1] if prefix else "item",
            "required": required,
            "type": "array",
            "description": schema.get("description", ""),
            "example": schema.get("example"),
            "format": schema.get("format"),
            "enum": schema.get("enum"),
            "minimum": schema.get("minimum"),
            "maximum": schema.get("maximum"),
            "exclusiveMinimum": schema.get("exclusiveMinimum"),
            "exclusiveMaximum": schema.get("exclusiveMaximum"),
            "minLength": schema.get("minLength"),
            "maxLength": schema.get("maxLength"),
            "pattern": schema.get("pattern"),
            "x-numeric-only": schema.get("x-numeric-only", False),
            "x-format-hint": schema.get("x-format-hint"),
        }]

    return [{
        "path": prefix,
        "name": prefix.split(".")[-1] if prefix else schema.get("title", "value"),
        "required": required,
        "type": schema.get("type", "string"),
        "description": schema.get("description", ""),
        "example": schema.get("example"),
        "format": schema.get("format"),
        "enum": schema.get("enum"),
        "minimum": schema.get("minimum"),
        "maximum": schema.get("maximum"),
        "exclusiveMinimum": schema.get("exclusiveMinimum"),
        "exclusiveMaximum": schema.get("exclusiveMaximum"),
        "minLength": schema.get("minLength"),
        "maxLength": schema.get("maxLength"),
        "pattern": schema.get("pattern"),
        "x-numeric-only": schema.get("x-numeric-only", False),
        "x-format-hint": schema.get("x-format-hint"),
    }]


def convert_parameter(param: dict, components: dict[str, dict]) -> dict:
    if "schema" in param:
        schema = normalize_schema(param["schema"], components)
        converted = {
            "name": param.get("name", ""),
            "in": param.get("in", ""),
            "required": bool(param.get("required")),
            "description": param.get("description", ""),
            "schema": schema,
            "type": (schema or {}).get("type"),
            "format": (schema or {}).get("format"),
            "example": (schema or {}).get("example"),
            "enum": (schema or {}).get("enum"),
        }
    else:
        converted = {
            "name": param.get("name", ""),
            "in": param.get("in", ""),
            "required": bool(param.get("required")),
            "description": param.get("description", ""),
            "type": param.get("type", "string"),
            "format": param.get("format"),
            "example": param.get("example"),
            "enum": param.get("enum"),
            "minimum": param.get("minimum"),
            "maximum": param.get("maximum"),
            "exclusiveMinimum": param.get("exclusiveMinimum"),
            "exclusiveMaximum": param.get("exclusiveMaximum"),
            "minLength": param.get("minLength"),
            "maxLength": param.get("maxLength"),
            "pattern": param.get("pattern"),
        }
        converted.update(extract_constraints_from_description(converted["description"]))
    return converted


def discover_success_code(components: dict[str, dict]) -> str | int | None:
    for schema in components.values():
        properties = schema.get("properties") or {}
        if "code" in properties and properties["code"].get("example") is not None:
            return properties["code"]["example"]
    return None


def extract_common_rules(data: dict, components: dict[str, dict]) -> dict:
    info = data.get("info") or {}
    description_raw = info.get("description", "")
    description_text = strip_html(description_raw)

    base_urls: list[str] = []
    if is_openapi3(data):
        for server in data.get("servers") or []:
            if server.get("url"):
                base_urls.append(server["url"])
    elif data.get("host"):
        scheme = (data.get("schemes") or ["http"])[0]
        base_path = data.get("basePath", "") or ""
        base_urls.append(f"{scheme}://{data['host']}{base_path}")

    for url in re.findall(r"https?://[^\s<>\"]+", description_raw):
        base_urls.append(trim_url(url))

    header_rules: list[dict] = []
    if "X-Lemonban-Media-Type" in description_text:
        header_rules.append({
            "name": "X-Lemonban-Media-Type",
            "allowed_values": ["lemonban.v1", "lemonban.v2", "lemonban.v3"],
            "rule": "v1 无鉴权，v2 token 鉴权，v3 timestamp+token+sign 鉴权。",
        })
    if "Content-Type" in description_text:
        header_rules.append({
            "name": "Content-Type",
            "allowed_values": ["application/json"],
            "rule": "POST、PUT、PATCH JSON 接口需要设置；GET 默认不设置。",
        })
    if "Authorization" in description_text:
        header_rules.append({
            "name": "Authorization",
            "allowed_values": ["Bearer ${token}"],
            "rule": "当使用 token 鉴权时必须携带，格式为 Bearer token_value。",
        })

    response_fields = ["code", "msg", "data"] if all(key in description_text for key in ("code", "msg", "data")) else []
    return {
        "description_text": description_text,
        "base_urls": unique_list([item for item in base_urls if item]),
        "headers": header_rules,
        "response_fields": response_fields,
        "success_code": discover_success_code(components),
        "return_code_references": unique_list(trim_url(item) for item in re.findall(r"https?://[^\s<>\"]+", description_text) if "返回码" in description_text),
    }


def build_path_regex(path_template: str) -> re.Pattern[str]:
    pattern = re.sub(r"\{[^/]+\}", r"[^/]+", path_template)
    return re.compile(rf"^{pattern}$")


def pick_primary_response(responses: dict, components: dict[str, dict]) -> dict:
    if not responses:
        return {}
    ordered_statuses = sorted(responses.keys(), key=lambda item: (not str(item).startswith("2"), str(item)))
    status = ordered_statuses[0]
    meta = responses[status] or {}
    schema = None
    if "schema" in meta:
        schema = normalize_schema(meta.get("schema"), components)
    elif "content" in meta:
        content = meta.get("content") or {}
        for item in content.values():
            if item.get("schema"):
                schema = normalize_schema(item["schema"], components)
                break
    return {
        "status": str(status),
        "description": meta.get("description", ""),
        "schema": schema,
    }


def merge_parameters(path_level: list[dict], operation_level: list[dict]) -> list[dict]:
    merged: list[dict] = []
    seen = set()
    for item in list(path_level or []) + list(operation_level or []):
        key = (item.get("name"), item.get("in"))
        if key in seen:
            continue
        seen.add(key)
        merged.append(item)
    return merged


def infer_special_flags(summary: str, path: str, method: str, consumes: list[str], form_data: list[dict]) -> dict:
    text = f"{summary} {path}"
    return {
        "public_candidate": any(keyword in text for keyword in PUBLIC_KEYWORDS),
        "login": "登录" in summary or path.endswith("/login"),
        "register": "注册" in summary or path.endswith("/register"),
        "verification_code": "验证码" in summary or "verificationCode" in path,
        "upload": "上传" in summary or "multipart/form-data" in consumes or any(item.get("type") == "file" for item in form_data),
        "list": "列表" in summary or "/list" in path or method == "GET",
        "state_flow": any(keyword in text for keyword in STATE_KEYWORDS),
    }


def detect_dependencies(endpoint: dict) -> list[str]:
    dependency_names: list[str] = []
    for field in endpoint.get("body_fields", []):
        if field["name"] in {"page", "size"}:
            continue
        if field["name"].endswith(ID_FIELD_SUFFIXES) and field["name"] not in {"id"}:
            dependency_names.append(field["name"])
    for field in endpoint.get("query_params", []) + endpoint.get("path_params", []):
        if field["name"].endswith(ID_FIELD_SUFFIXES) and field["name"] not in {"id"}:
            dependency_names.append(field["name"])
    return unique_list(dependency_names)


def infer_auth(endpoint: dict, top_level_security: list | None) -> dict:
    security = endpoint.get("security")
    header_names = {item["name"] for item in endpoint.get("header_params", [])}
    explicit_required = bool(security or top_level_security)
    public_candidate = endpoint["special_flags"]["public_candidate"]
    has_auth_headers = "Authorization" in header_names or "X-Lemonban-Media-Type" in header_names
    inferred_required = explicit_required or (has_auth_headers and not public_candidate)
    media_type = "lemonban.v1" if not inferred_required else "lemonban.v3"
    ambiguity = bool(has_auth_headers and not explicit_required and not public_candidate)
    reason = ""
    if explicit_required:
        reason = "契约存在显式 security 声明。"
    elif public_candidate:
        reason = "接口名称/路径呈现登录、注册或验证码特征，默认按公共接口生成。"
    elif inferred_required:
        reason = "接口包含 Authorization/X-Lemonban-Media-Type 头，默认按业务接口生成。"
    return {
        "requires_authorization": inferred_required,
        "media_type": media_type,
        "ambiguity": ambiguity,
        "reason": reason,
    }


def normalize_swagger_like(data: dict) -> dict:
    components = get_schema_components(data)
    common_rules = extract_common_rules(data, components)
    top_level_security = data.get("security")
    endpoints: list[dict] = []

    for path, path_meta in (data.get("paths") or {}).items():
        path_parameters = path_meta.get("parameters") or []
        for method, meta in path_meta.items():
            if method.lower() not in {"get", "post", "put", "patch", "delete", "options", "head"}:
                continue
            params = merge_parameters(path_parameters, meta.get("parameters") or [])
            header_params: list[dict] = []
            query_params: list[dict] = []
            path_params: list[dict] = []
            form_data: list[dict] = []
            body: dict | None = None

            for raw_param in params:
                param = convert_parameter(raw_param, components)
                location = param.get("in")
                if location == "header":
                    header_params.append(param)
                elif location == "query":
                    query_params.append(param)
                elif location == "path":
                    path_params.append(param)
                elif location == "formData":
                    form_data.append(param)
                elif location == "body":
                    body = {
                        "name": param.get("name", "body"),
                        "required": bool(raw_param.get("required")),
                        "schema": param.get("schema"),
                    }

            if is_openapi3(data):
                request_body = meta.get("requestBody") or {}
                if request_body:
                    content = request_body.get("content") or {}
                    body_schema = None
                    content_type = ""
                    for candidate_content_type, payload in content.items():
                        if payload.get("schema"):
                            body_schema = normalize_schema(payload["schema"], components)
                            content_type = candidate_content_type
                            break
                    if body_schema is not None:
                        body = {
                            "name": "body",
                            "required": bool(request_body.get("required")),
                            "schema": body_schema,
                            "content_type": content_type,
                        }

            consumes = meta.get("consumes") or []
            if body and not body.get("content_type"):
                body["content_type"] = consumes[0] if consumes else "application/json"

            response = pick_primary_response(meta.get("responses") or {}, components)
            summary = clean_summary(meta.get("summary", ""))
            endpoint = {
                "method": method.upper(),
                "path": path,
                "path_regex": build_path_regex(path).pattern,
                "summary": summary,
                "operationId": meta.get("operationId", ""),
                "tags": meta.get("tags", []),
                "consumes": consumes,
                "produces": meta.get("produces") or [],
                "header_params": header_params,
                "query_params": query_params,
                "path_params": path_params,
                "form_data": form_data,
                "body": body,
                "response": response,
                "security": meta.get("security"),
            }
            endpoint["body_fields"] = flatten_schema_fields((body or {}).get("schema"))
            endpoint["response_fields"] = flatten_schema_fields((response or {}).get("schema"))
            endpoint["special_flags"] = infer_special_flags(summary, path, method.upper(), consumes, form_data)
            endpoint["dependencies"] = detect_dependencies(endpoint)
            endpoint["auth"] = infer_auth(endpoint, top_level_security)
            endpoints.append(endpoint)

    title = (data.get("info") or {}).get("title", "")
    version = (data.get("info") or {}).get("version", "")
    return {
        "title": title,
        "version": version,
        "host": data.get("host", ""),
        "basePath": data.get("basePath", ""),
        "description": strip_html((data.get("info") or {}).get("description", "")),
        "tags": [tag.get("name", "") for tag in data.get("tags", [])],
        "components": {"schema_count": len(components)},
        "common_rules": common_rules,
        "endpoints": endpoints,
    }


def normalize_plain_text(text: str) -> dict:
    endpoints = []
    current = None
    pattern = re.compile(r"^(GET|POST|PUT|PATCH|DELETE)\s+(\S+)", re.I)
    for raw_line in text.splitlines():
        line = raw_line.strip()
        match = pattern.match(line)
        if match:
            if current:
                current["auth"] = {"requires_authorization": False, "media_type": "lemonban.v1", "ambiguity": True, "reason": "纯文本契约未提供明确鉴权规则。"}
                current["special_flags"] = infer_special_flags(current["summary"], current["path"], current["method"], [], [])
                current["dependencies"] = []
                current["body_fields"] = []
                current["response_fields"] = []
                endpoints.append(current)
            current = {
                "method": match.group(1).upper(),
                "path": match.group(2),
                "path_regex": build_path_regex(match.group(2)).pattern,
                "summary": "",
                "operationId": "",
                "tags": [],
                "consumes": [],
                "produces": [],
                "header_params": [],
                "query_params": [],
                "path_params": [],
                "form_data": [],
                "body": None,
                "response": {},
                "security": None,
            }
            continue
        if current and not current["summary"] and line:
            current["summary"] = line

    if current:
        current["auth"] = {"requires_authorization": False, "media_type": "lemonban.v1", "ambiguity": True, "reason": "纯文本契约未提供明确鉴权规则。"}
        current["special_flags"] = infer_special_flags(current["summary"], current["path"], current["method"], [], [])
        current["dependencies"] = []
        current["body_fields"] = []
        current["response_fields"] = []
        endpoints.append(current)

    return {
        "title": "",
        "version": "",
        "host": "",
        "basePath": "",
        "description": text.strip(),
        "tags": [],
        "components": {"schema_count": 0},
        "common_rules": {
            "description_text": text.strip(),
            "base_urls": [],
            "headers": [],
            "response_fields": [],
            "success_code": None,
            "return_code_references": [],
        },
        "endpoints": endpoints,
    }


def normalize_contract(path: Path) -> dict:
    fmt, payload = parse_contract(path)
    if fmt in {"swagger-json", "swagger-yaml"}:
        normalized = normalize_swagger_like(payload)  # type: ignore[arg-type]
    else:
        normalized = normalize_plain_text(payload)  # type: ignore[arg-type]
    return {"format": fmt, "normalized": normalized}


def field_display_name(field: dict) -> str:
    return field.get("description") or field.get("name") or field.get("path") or "字段"


def scalar_from_example(example: Any) -> Any:
    if isinstance(example, str):
        return example
    if isinstance(example, (int, float, bool)):
        return example
    return None


def default_string_value(name: str, description: str = "", min_length: int | None = None, max_length: int | None = None, numeric_only: bool = False, format_hint: str | None = None) -> str:
    lowered = name.lower()
    if format_hint == "phone" or "phone" in lowered or "手机号" in description:
        value = "18600000000"
    elif format_hint == "email" or "email" in lowered or "邮箱" in description:
        value = "test@example.com"
    elif format_hint == "id-card" or "idcard" in lowered or "身份证" in description:
        value = "110101199001011234"
    elif "token" in lowered:
        value = "${token}"
    elif "code" in lowered:
        value = "888888"
    elif numeric_only or "num" in lowered or "编号" in description:
        value = "001"
    elif "name" in lowered or "名称" in description:
        value = "测试名称"
    elif "address" in lowered or "地址" in description:
        value = "测试地址1号"
    elif "remark" in lowered or "备注" in description:
        value = "测试备注"
    elif "nearby" in lowered or "地标" in description:
        value = "测试地标"
    elif "password" in lowered or "密码" in description:
        value = "Aa123456"
    elif format_hint == "alnum-mixed":
        value = "Aa123456"
    else:
        value = "测试值"

    if max_length is not None and len(value) > max_length:
        value = value[:max_length]
    if min_length is not None and len(value) < min_length:
        value = value + ("X" * (min_length - len(value)))
    return value


def valid_scalar_for_field(field: dict) -> Any:
    example = scalar_from_example(field.get("example"))
    if example is not None and not validate_value_against_field(example, {**field, "required": False}):
        return example
    if field.get("enum"):
        return field["enum"][0]

    field_type = field.get("type", "string")
    minimum = field.get("minimum")
    exclusive_minimum = field.get("exclusiveMinimum")
    if field_type == "integer":
        if minimum is not None:
            return int(minimum + 1 if exclusive_minimum else minimum)
        return 1
    if field_type == "number":
        if minimum is not None:
            return float(minimum + 1 if exclusive_minimum else minimum)
        return 1.23
    if field_type == "boolean":
        return True
    if field_type == "array":
        return []
    return default_string_value(
        name=field.get("name", ""),
        description=field.get("description", ""),
        min_length=field.get("minLength"),
        max_length=field.get("maxLength"),
        numeric_only=bool(field.get("x-numeric-only")),
        format_hint=field.get("x-format-hint") or field.get("format"),
    )


def build_example_from_schema(schema: dict | None) -> Any:
    if not schema:
        return None
    schema_type = schema.get("type")
    if schema_type == "object" or schema.get("properties"):
        result = {}
        required = set(schema.get("required") or [])
        for name, prop in (schema.get("properties") or {}).items():
            if required or prop.get("example") is not None or prop.get("description"):
                result[name] = build_example_from_schema(prop)
        return result
    if schema_type == "array":
        return [build_example_from_schema(schema.get("items") or {})]
    field = {
        "name": schema.get("title", "value"),
        "type": schema.get("type", "string"),
        "description": schema.get("description", ""),
        "example": schema.get("example"),
        "format": schema.get("format"),
        "enum": schema.get("enum"),
        "minimum": schema.get("minimum"),
        "maximum": schema.get("maximum"),
        "exclusiveMinimum": schema.get("exclusiveMinimum"),
        "exclusiveMaximum": schema.get("exclusiveMaximum"),
        "minLength": schema.get("minLength"),
        "maxLength": schema.get("maxLength"),
        "pattern": schema.get("pattern"),
        "x-numeric-only": schema.get("x-numeric-only", False),
        "x-format-hint": schema.get("x-format-hint"),
    }
    return valid_scalar_for_field(field)


def set_nested_value(payload: Any, path: str, value: Any) -> Any:
    if not path:
        return value
    parts = path.split(".")
    cursor = payload
    for part in parts[:-1]:
        if part.endswith("[0]"):
            key = part[:-3]
            cursor = cursor.setdefault(key, [{}])
            if not cursor:
                cursor.append({})
            cursor = cursor[0]
        else:
            cursor = cursor.setdefault(part, {})
    last = parts[-1]
    if last.endswith("[0]"):
        key = last[:-3]
        cursor[key] = [value]
    else:
        cursor[last] = value
    return payload


def delete_nested_value(payload: Any, path: str) -> Any:
    parts = path.split(".")
    cursor = payload
    for part in parts[:-1]:
        key = part[:-3] if part.endswith("[0]") else part
        if isinstance(cursor, dict):
            cursor = cursor.get(key)
        elif isinstance(cursor, list):
            cursor = cursor[0] if cursor else None
        if cursor is None:
            return payload
    last = parts[-1]
    key = last[:-3] if last.endswith("[0]") else last
    if isinstance(cursor, dict):
        cursor.pop(key, None)
    return payload


def invalid_value_for_field(field: dict, strategy: str) -> Any:
    field_type = field.get("type", "string")
    if strategy == "missing":
        return None
    if strategy == "invalid_enum":
        return "__invalid_enum__"
    if strategy == "too_long":
        max_length = field.get("maxLength") or 1
        return "X" * (int(max_length) + 1)
    if strategy == "too_short":
        return ""
    if strategy == "non_numeric":
        return "abc"
    if strategy == "out_of_range":
        if field_type in {"integer", "number"}:
            minimum = field.get("minimum")
            maximum = field.get("maximum")
            if minimum is not None:
                return minimum - 1
            if maximum is not None:
                return maximum + 1
            return -1
        return ""
    if strategy == "wrong_type":
        if field_type == "file":
            return "test.exe"
        if field_type in {"integer", "number"}:
            return "abc"
        if field_type == "string":
            return 12345
        if field_type == "boolean":
            return "abc"
        return None
    return None


def select_boundary_target(endpoint: dict) -> tuple[str, dict] | None:
    candidates = endpoint.get("body_fields", []) + endpoint.get("form_data", []) + endpoint.get("query_params", []) + endpoint.get("path_params", [])
    for field in candidates:
        if field.get("type") == "file":
            return "wrong_type", field
        if field.get("enum"):
            return "invalid_enum", field
        if field.get("maxLength") is not None:
            return "too_long", field
        if field.get("minLength") not in (None, 0):
            return "too_short", field
        if field.get("x-numeric-only"):
            return "non_numeric", field
        if field.get("minimum") is not None or field.get("maximum") is not None:
            return "out_of_range", field
    for field in candidates:
        if field.get("type") in {"integer", "number", "string"}:
            return "wrong_type", field
    return None


def choose_required_target(endpoint: dict) -> tuple[str, dict] | None:
    for field in endpoint.get("body_fields", []):
        if field.get("required"):
            return "missing_body", field
    for field in endpoint.get("form_data", []):
        if field.get("required"):
            return "missing_form", field
    for field in endpoint.get("path_params", []) + endpoint.get("query_params", []):
        if field.get("required"):
            return "invalid_param", field
    return None


def default_headers_for_endpoint(endpoint: dict) -> dict[str, str]:
    headers: dict[str, str] = {}
    auth = endpoint.get("auth") or {}
    media_type = auth.get("media_type")
    if media_type:
        headers["X-Lemonban-Media-Type"] = media_type
    has_body = bool(endpoint.get("body"))
    if has_body and endpoint["method"] in {"POST", "PUT", "PATCH"} and not endpoint["special_flags"]["upload"]:
        headers["Content-Type"] = "application/json"
    if auth.get("requires_authorization"):
        headers["Authorization"] = "Bearer ${token}"
        if media_type == "lemonban.v3":
            headers["timestamp"] = "${timestamp}"
            headers["sign"] = "${sign}"
    return headers


def valid_request_payload(endpoint: dict) -> dict:
    path_values = {field["name"]: valid_scalar_for_field(field) for field in endpoint.get("path_params", [])}
    query_values = {}
    for field in endpoint.get("query_params", []):
        if field.get("required"):
            query_values[field["name"]] = valid_scalar_for_field(field)
    body_value = ""
    if endpoint.get("body"):
        body_value = build_example_from_schema(endpoint["body"]["schema"])
    elif endpoint.get("form_data"):
        body_value = {}
        for field in endpoint["form_data"]:
            if field.get("type") == "file":
                body_value[field["name"]] = "test.jpg"
            elif field.get("required"):
                body_value[field["name"]] = valid_scalar_for_field(field)
    return {
        "path_values": path_values,
        "query_values": query_values,
        "body": body_value,
        "headers": default_headers_for_endpoint(endpoint),
    }


def render_actual_path(endpoint: dict, path_values: dict, query_values: dict | None = None) -> str:
    path = endpoint["path"]
    for field in endpoint.get("path_params", []):
        placeholder = "{" + field["name"] + "}"
        path = path.replace(placeholder, str(path_values.get(field["name"], valid_scalar_for_field(field))))
    if query_values:
        encoded = urlencode(query_values, doseq=True)
        if encoded:
            path = f"{path}?{encoded}"
    return path


def detect_data_assert_path(endpoint: dict) -> str | None:
    response_fields = endpoint.get("response_fields", [])
    token_candidates = [field["path"] for field in response_fields if "token" in field["path"].lower()]
    if token_candidates:
        return token_candidates[0]
    id_candidates = [field["path"] for field in response_fields if field["path"].lower().endswith(".id") or field["path"].lower() == "data.id"]
    if id_candidates:
        return id_candidates[0]
    for field in response_fields:
        if field["path"].startswith("data."):
            return field["path"]
    if any(field["path"] == "data" for field in response_fields):
        return "data"
    return None


def format_literal(value: Any) -> str:
    if isinstance(value, bool):
        return "true" if value else "false"
    if value is None:
        return "null"
    if isinstance(value, (int, float)):
        return str(value)
    return json.dumps(str(value), ensure_ascii=False)


def positive_assertions(endpoint: dict, success_code: Any) -> str:
    response = endpoint.get("response") or {}
    assertions = [f"http_status == {response.get('status', '200')}"]
    response_fields = endpoint.get("response_fields", [])
    response_paths = {field["path"] for field in response_fields}
    if "code" in response_paths:
        if success_code is not None:
            assertions.append(f"$.code == {format_literal(success_code)}")
        else:
            assertions.append("$.code != null")
    if "msg" in response_paths:
        assertions.append('$.msg contains "成功"')
    data_path = detect_data_assert_path(endpoint)
    if data_path:
        assertions.append(f"$.{data_path} != null")
    elif "data" in response_paths:
        assertions.append("$.data != null")
    return "; ".join(unique_list(assertions))


def negative_assertions(endpoint: dict, success_code: Any) -> str:
    response = endpoint.get("response") or {}
    assertions = [f"http_status == {response.get('status', '200')}"]
    response_paths = {field["path"] for field in endpoint.get("response_fields", [])}
    if "code" in response_paths:
        if success_code is not None:
            assertions.append(f"$.code != {format_literal(success_code)}")
        else:
            assertions.append("$.code != null")
    if "msg" in response_paths:
        assertions.append("$.msg != null")
    return "; ".join(unique_list(assertions))


def serialize_json_field(value: Any) -> str:
    if value == "" or value is None:
        return ""
    return json.dumps(value, ensure_ascii=False, separators=(",", ":"))


def join_prerequisite_steps(steps: list[str]) -> str:
    clean_steps = [step.strip().rstrip("。") for step in steps if step and step.strip()]
    if not clean_steps:
        return ""
    return "\n".join(f"{index}、{step}。" for index, step in enumerate(clean_steps, 1))


def endpoint_reference(contract: dict, path: str, fallback_name: str) -> str:
    for endpoint in contract.get("normalized", {}).get("endpoints", []):
        if endpoint.get("path") == path:
            return f"{endpoint['summary']}接口 `{endpoint['method']} {endpoint['path']}`"
    return f"{fallback_name} `{path}`"


def choose_login_path(endpoint: dict) -> str:
    path = endpoint.get("path", "")
    if path.startswith("/owner/room") or path.startswith("/rent/"):
        return "/owner/login"
    return "/user/login"


def choose_login_reference(endpoint: dict, contract: dict) -> str:
    login_path = choose_login_path(endpoint)
    if login_path == "/owner/login":
        return endpoint_reference(contract, "/owner/login", "业主登录接口")
    return endpoint_reference(contract, "/user/login", "管理端用户登录接口")


def public_prerequisite_paths(endpoint: dict) -> list[str]:
    paths: list[str] = []
    path = endpoint.get("path", "")

    if endpoint["special_flags"]["register"]:
        paths.append("/verificationCode/message")

    if endpoint["special_flags"]["verification_code"]:
        return unique_list(paths)

    if endpoint["special_flags"]["login"]:
        if path == "/user/login":
            paths.extend(["/user/register", "/verificationCode/message"])
        return unique_list(paths)

    if endpoint.get("auth", {}).get("requires_authorization"):
        login_path = choose_login_path(endpoint)
        paths.append(login_path)
        if login_path == "/user/login":
            paths.extend(["/user/register", "/verificationCode/message"])

    return unique_list(paths)


def endpoint_identity(endpoint: dict) -> tuple[str, str]:
    return (str(endpoint.get("method", "")).upper(), str(endpoint.get("path", "")))


def insert_endpoint_before(target: list[dict], endpoint: dict, before_identity: tuple[str, str] | None) -> None:
    if before_identity is None:
        target.append(endpoint)
        return
    for idx, current in enumerate(target):
        if endpoint_identity(current) == before_identity:
            target.insert(idx, endpoint)
            return
    target.append(endpoint)


def include_prerequisite_public_endpoints(source_contract: dict, selected_contract: dict) -> dict:
    enriched = copy.deepcopy(selected_contract)
    source_lookup = {
        endpoint.get("path"): endpoint
        for endpoint in source_contract.get("normalized", {}).get("endpoints", [])
        if endpoint.get("path") in PUBLIC_PREREQUISITE_PATHS
    }
    endpoints = enriched.get("normalized", {}).get("endpoints", [])
    existing = {endpoint_identity(endpoint) for endpoint in endpoints}
    queue = list(endpoints)
    auto_included: list[dict[str, str]] = []
    index = 0

    while index < len(queue):
        endpoint = queue[index]
        index += 1
        for prerequisite_path in public_prerequisite_paths(endpoint):
            prerequisite = source_lookup.get(prerequisite_path)
            if prerequisite is None:
                continue
            identity = endpoint_identity(prerequisite)
            if identity in existing:
                continue
            cloned = copy.deepcopy(prerequisite)
            insert_endpoint_before(endpoints, cloned, endpoint_identity(endpoint))
            queue.append(cloned)
            existing.add(identity)
            auto_included.append({
                "summary": cloned["summary"],
                "method": cloned["method"],
                "path": cloned["path"],
            })

    enriched["normalized"]["tags"] = unique_list([
        tag
        for endpoint in endpoints
        for tag in endpoint.get("tags", [])
        if tag
    ])
    enriched["normalized"]["auto_included_endpoints"] = auto_included
    return enriched


def account_bootstrap_steps(endpoint: dict, contract: dict) -> list[str]:
    path = endpoint.get("path", "")
    if endpoint["special_flags"]["register"]:
        verification_ref = endpoint_reference(contract, "/verificationCode/message", "获取短信验证码接口")
        return [
            f"先调用{verification_ref}获取有效验证码",
            "准备未注册手机号、用户名和密码",
        ]
    if path == "/user/login":
        register_ref = endpoint_reference(contract, "/user/register", "用户注册接口")
        return [
            f"如需新账号，先调用{register_ref}完成注册",
            "准备已注册的管理端账号和密码",
        ]
    if path == "/owner/login":
        bind_ref = endpoint_reference(contract, "/owner/bind", "绑定业主接口")
        return [
            f"如需开通业主登录账号，先调用{bind_ref}完成绑定",
            "准备已绑定的业主手机号和密码",
        ]
    return []


def auth_prerequisite_steps(endpoint: dict, contract: dict) -> list[str]:
    auth = endpoint.get("auth") or {}
    if not auth.get("requires_authorization"):
        return []
    media_type = auth.get("media_type")
    login_ref = choose_login_reference(endpoint, contract)
    if media_type == "lemonban.v3":
        return [
            f"调用{login_ref}获取 token；基于 token 生成 timestamp 和 sign",
            "headers 使用 X-Lemonban-Media-Type=lemonban.v3、Authorization=Bearer ${token}、timestamp、sign",
        ]
    return [
        f"调用{login_ref}获取 token",
        "headers 使用 Authorization=Bearer ${token}",
    ]


def positive_prerequisite(endpoint: dict, contract: dict) -> str:
    if endpoint["special_flags"]["login"] or endpoint["special_flags"]["register"]:
        return join_prerequisite_steps(account_bootstrap_steps(endpoint, contract))
    if endpoint["special_flags"]["verification_code"]:
        return join_prerequisite_steps(["准备一个未注册或可正常接收验证码的手机号"])
    steps = []
    if endpoint.get("auth", {}).get("requires_authorization"):
        steps.extend(account_bootstrap_steps(endpoint, contract))
    steps.extend(auth_prerequisite_steps(endpoint, contract))
    if endpoint["dependencies"]:
        dependency_text = "、".join(endpoint["dependencies"])
        steps.append(f"并准备有效的上游资源数据：{dependency_text}")
    return join_prerequisite_steps(steps)


def negative_prerequisite(endpoint: dict, contract: dict, scene: str) -> str:
    if "缺少鉴权头" in scene and endpoint.get("auth", {}).get("requires_authorization"):
        steps = account_bootstrap_steps(endpoint, contract)
        steps.append(auth_prerequisite_steps(endpoint, contract)[0] if auth_prerequisite_steps(endpoint, contract) else "先获取有效 token")
        steps.append("本条用例故意不传 Authorization 或相关鉴权字段")
        return join_prerequisite_steps(steps)
    if endpoint["special_flags"]["register"]:
        return join_prerequisite_steps([
            f"先调用{endpoint_reference(contract, '/verificationCode/message', '获取短信验证码接口')}获取有效验证码",
            "准备未注册手机号，按场景修改其中一个参数使其非法",
        ])
    if endpoint["special_flags"]["login"]:
        return join_prerequisite_steps([
            *account_bootstrap_steps(endpoint, contract),
            "按场景修改其中一个参数使其非法",
        ])
    return positive_prerequisite(endpoint, contract)


def make_case(interface_name: str, prerequisite: str, scene: str, path: str, headers: dict[str, Any], body: Any, expected: str, assertions: str) -> dict:
    return {
        "接口中文名称": interface_name,
        "前置条件": prerequisite,
        "测试场景": scene,
        "path": path,
        "headers": serialize_json_field(headers),
        "body": serialize_json_field(body),
        "预期响应结果": expected,
        "断言响应": assertions,
        "断言sql": "",
    }


def positive_expected(endpoint: dict) -> str:
    name = endpoint["summary"]
    if endpoint["special_flags"]["login"]:
        return f"{name}成功，返回成功码，data 返回登录结果或 token 信息。"
    if endpoint["special_flags"]["verification_code"]:
        return f"{name}成功，返回成功码，data 返回验证码发送结果。"
    if endpoint["special_flags"]["upload"]:
        return f"{name}成功，返回成功码，data 返回文件地址或文件标识。"
    if endpoint["special_flags"]["list"]:
        return f"{name}成功，返回成功码，data 返回列表或分页数据。"
    return f"{name}成功，返回成功码，msg 提示成功，data 返回本次操作结果。"


def required_field_display(field: dict) -> str:
    return field.get("name") or field.get("path") or "字段"


def generate_endpoint_cases(endpoint: dict, contract: dict) -> list[dict]:
    success_code = (contract["normalized"]["common_rules"] or {}).get("success_code")
    interface_name = endpoint["summary"]
    base_request = valid_request_payload(endpoint)
    cases: list[dict] = []

    cases.append(make_case(
        interface_name=interface_name,
        prerequisite=positive_prerequisite(endpoint, contract),
        scene=f"P0_正向_{interface_name}成功",
        path=render_actual_path(endpoint, base_request["path_values"], base_request["query_values"]),
        headers=base_request["headers"],
        body=base_request["body"],
        expected=positive_expected(endpoint),
        assertions=positive_assertions(endpoint, success_code),
    ))

    auth = endpoint.get("auth") or {}
    if auth.get("requires_authorization"):
        invalid_headers = copy.deepcopy(base_request["headers"])
        invalid_headers.pop("Authorization", None)
        cases.append(make_case(
            interface_name=interface_name,
            prerequisite=negative_prerequisite(endpoint, contract, "缺少鉴权头"),
            scene=f"P1_异常_缺少鉴权头",
            path=render_actual_path(endpoint, base_request["path_values"], base_request["query_values"]),
            headers=invalid_headers,
            body=base_request["body"],
            expected="接口拒绝未鉴权请求，返回失败响应，code 不为成功码，msg 提示鉴权失败。",
            assertions=negative_assertions(endpoint, success_code),
        ))
    else:
        required_target = choose_required_target(endpoint)
        if required_target:
            kind, field = required_target
            request = copy.deepcopy(base_request)
            if kind == "missing_body" and isinstance(request["body"], dict):
                delete_nested_value(request["body"], field["path"])
            elif kind == "missing_form" and isinstance(request["body"], dict):
                request["body"].pop(field["name"], None)
            elif kind == "invalid_param":
                invalid_value = invalid_value_for_field(field, "wrong_type")
                if any(param["name"] == field["name"] for param in endpoint.get("query_params", [])):
                    request["query_values"][field["name"]] = invalid_value
                else:
                    request["path_values"][field["name"]] = invalid_value
            cases.append(make_case(
                interface_name=interface_name,
                prerequisite=negative_prerequisite(endpoint, contract, f"{required_field_display(field)}非法或缺失"),
                scene=f"P1_异常_{required_field_display(field)}非法或缺失",
                path=render_actual_path(endpoint, request["path_values"], request["query_values"]),
                headers=request["headers"],
                body=request["body"],
                expected="接口拒绝非法请求，返回失败响应，code 不为成功码，msg 提示参数校验失败。",
                assertions=negative_assertions(endpoint, success_code),
            ))

    boundary_target = select_boundary_target(endpoint)
    if boundary_target:
        strategy, field = boundary_target
        request = copy.deepcopy(base_request)
        invalid_value = invalid_value_for_field(field, strategy)
        if field.get("path") and field["path"] in {item["path"] for item in endpoint.get("body_fields", [])} and isinstance(request["body"], dict):
            set_nested_value(request["body"], field["path"], invalid_value)
        elif any(param["name"] == field["name"] for param in endpoint.get("form_data", [])) and isinstance(request["body"], dict):
            request["body"][field["name"]] = invalid_value
        elif any(param["name"] == field["name"] for param in endpoint.get("query_params", [])):
            request["query_values"][field["name"]] = invalid_value
        else:
            request["path_values"][field["name"]] = invalid_value
        cases.append(make_case(
            interface_name=interface_name,
            prerequisite=negative_prerequisite(endpoint, contract, f"{required_field_display(field)}校验"),
            scene=f"P1_边界_{required_field_display(field)}校验",
            path=render_actual_path(endpoint, request["path_values"], request["query_values"]),
            headers=request["headers"],
            body=request["body"],
            expected="接口拒绝越界或非法取值，返回失败响应，code 不为成功码，msg 提示参数校验失败。",
            assertions=negative_assertions(endpoint, success_code),
        ))

    if endpoint["special_flags"]["state_flow"]:
        cases.append(make_case(
            interface_name=interface_name,
            prerequisite=positive_prerequisite(endpoint, contract),
            scene="P1_状态流转_重复操作或非法状态拦截",
            path=render_actual_path(endpoint, base_request["path_values"], base_request["query_values"]),
            headers=base_request["headers"],
            body=base_request["body"],
            expected="当资源已处理、已绑定、已审核或状态不允许时，接口应拒绝重复或非法状态流转请求。",
            assertions=negative_assertions(endpoint, success_code),
        ))
    elif endpoint["dependencies"]:
        dependency_name = endpoint["dependencies"][0]
        request = copy.deepcopy(base_request)
        if isinstance(request["body"], dict):
            set_nested_value(request["body"], dependency_name, 99999999)
        elif dependency_name in request["query_values"]:
            request["query_values"][dependency_name] = 99999999
        else:
            request["path_values"][dependency_name] = 99999999
        cases.append(make_case(
            interface_name=interface_name,
            prerequisite=positive_prerequisite(endpoint, contract),
            scene=f"P1_流程_上游资源{dependency_name}不存在",
            path=render_actual_path(endpoint, request["path_values"], request["query_values"]),
            headers=request["headers"],
            body=request["body"],
            expected="当依赖资源不存在时，接口应拒绝请求并返回失败响应。",
            assertions=negative_assertions(endpoint, success_code),
        ))

    return unique_list(cases)


def summarize_key_fields(endpoint: dict) -> list[str]:
    result = []
    for field in endpoint.get("body_fields", [])[:4]:
        constraint_parts = []
        if field.get("required"):
            constraint_parts.append("必填")
        if field.get("maxLength") is not None:
            constraint_parts.append(f"长度<={field['maxLength']}")
        if field.get("minLength") is not None:
            constraint_parts.append(f"长度>={field['minLength']}")
        if field.get("enum"):
            constraint_parts.append(f"枚举={field['enum']}")
        if field.get("minimum") is not None:
            constraint_parts.append(f"最小值={field['minimum']}")
        label = field["name"]
        if constraint_parts:
            label += f"（{'，'.join(str(item) for item in constraint_parts)}）"
        result.append(label)
    if not result:
        for field in endpoint.get("form_data", [])[:4]:
            constraint_parts = []
            if field.get("required"):
                constraint_parts.append("必填")
            if field.get("x-file-exts"):
                constraint_parts.append(f"类型={'/'.join(field['x-file-exts'])}")
            if field.get("x-file-max-kb") is not None:
                constraint_parts.append(f"大小<={field['x-file-max-kb']}KB")
            label = field["name"]
            if constraint_parts:
                label += f"（{'，'.join(str(item) for item in constraint_parts)}）"
            result.append(label)
    if not result:
        for field in (endpoint.get("query_params", []) + endpoint.get("path_params", []))[:4]:
            label = field["name"]
            if field.get("required"):
                label += "（必填）"
            result.append(label)
    return result


def detect_pending_items(contract: dict) -> list[dict]:
    normalized = contract["normalized"]
    pending: list[dict] = []
    common_rules = normalized.get("common_rules") or {}

    if not common_rules.get("success_code"):
        pending.append({
            "接口中文名称": "全局",
            "原始契约描述": "契约未明确列出成功码。",
            "不明确点": "正向/异常场景无法做精确 code 断言。",
            "影响": "异常场景只能断言 code 不等于成功码或 msg 不为空。",
            "待确认问题": "请确认通用成功码与常见失败码取值。",
        })

    if not common_rules.get("return_code_references"):
        pending.append({
            "接口中文名称": "全局",
            "原始契约描述": "契约未直接给出完整通用返回码清单。",
            "不明确点": "异常场景缺少精确 code/msg 映射。",
            "影响": "二次审核只能判断是否缺少异常断言，不能校验精确错误码。",
            "待确认问题": "请补充通用返回码文档或错误码清单。",
        })

    for endpoint in normalized.get("endpoints", []):
        if endpoint.get("auth", {}).get("ambiguity"):
            pending.append({
                "接口中文名称": endpoint["summary"],
                "原始契约描述": endpoint["auth"]["reason"],
                "不明确点": "接口是否必须 token 鉴权、是否允许 v1/v2/v3 中的哪种版本未显式声明。",
                "影响": "已按默认业务接口生成用例，但真实执行前仍需确认鉴权版本。",
                "待确认问题": "请确认该接口的实际鉴权要求与请求头版本。",
            })

        if endpoint["special_flags"]["state_flow"]:
            pending.append({
                "接口中文名称": endpoint["summary"],
                "原始契约描述": endpoint["path"],
                "不明确点": "存在审核/绑定/入住/发布等状态动作，但契约未给出完整状态迁移规则。",
                "影响": "只能生成重复操作/非法状态流转的通用异常场景，无法断言精确业务提示。",
                "待确认问题": "请确认允许的前置状态、目标状态及重复操作提示。",
            })
    return unique_list(pending)


def endpoint_matches_filters(endpoint: dict, tag_filters: list[str] | None = None, interface_name_filters: list[str] | None = None, path_keywords: list[str] | None = None) -> bool:
    tag_filters = [item.lower() for item in (tag_filters or []) if item]
    interface_name_filters = [item.lower() for item in (interface_name_filters or []) if item]
    path_keywords = [item.lower() for item in (path_keywords or []) if item]

    endpoint_tags = [str(item).lower() for item in endpoint.get("tags", [])]
    summary = str(endpoint.get("summary", "")).lower()
    path = str(endpoint.get("path", "")).lower()

    if tag_filters and not any(filter_text in tag for filter_text in tag_filters for tag in endpoint_tags):
        return False
    if interface_name_filters and not any(filter_text in summary for filter_text in interface_name_filters):
        return False
    if path_keywords and not any(filter_text in path for filter_text in path_keywords):
        return False
    return True


def filter_contract(contract: dict, tag_filters: list[str] | None = None, interface_name_filters: list[str] | None = None, path_keywords: list[str] | None = None, include_prerequisites: bool = True) -> dict:
    filtered = copy.deepcopy(contract)
    endpoints = filtered["normalized"].get("endpoints", [])
    filtered["normalized"]["endpoints"] = [
        endpoint for endpoint in endpoints
        if endpoint_matches_filters(endpoint, tag_filters=tag_filters, interface_name_filters=interface_name_filters, path_keywords=path_keywords)
    ]
    filtered["normalized"]["tags"] = unique_list([
        tag
        for endpoint in filtered["normalized"]["endpoints"]
        for tag in endpoint.get("tags", [])
        if tag
    ])
    filtered["normalized"]["auto_included_endpoints"] = []
    if include_prerequisites:
        return include_prerequisite_public_endpoints(contract, filtered)
    return filtered


def grouped_contracts_by_tag(contract: dict, source_contract: dict | None = None) -> list[tuple[str, dict]]:
    source_contract = source_contract or contract
    endpoints = contract["normalized"].get("endpoints", [])
    grouped: dict[str, list[dict]] = defaultdict(list)
    for endpoint in endpoints:
        tag_names = endpoint.get("tags") or ["未分组接口"]
        for tag_name in tag_names:
            grouped[tag_name].append(endpoint)

    result = []
    for tag_name, tag_endpoints in grouped.items():
        cloned = copy.deepcopy(contract)
        cloned["normalized"]["endpoints"] = copy.deepcopy(tag_endpoints)
        cloned["normalized"]["tags"] = [tag_name]
        cloned["normalized"]["auto_included_endpoints"] = []
        result.append((tag_name, include_prerequisite_public_endpoints(source_contract, cloned)))
    return result


def slugify_text(text: str) -> str:
    slug = re.sub(r"[\\/:*?\"<>|]+", "-", text.strip())
    slug = re.sub(r"\s+", "-", slug)
    slug = re.sub(r"-{2,}", "-", slug).strip("-")
    return slug or "未命名"


def generate_testcase_payload(contract: dict) -> dict:
    normalized = contract["normalized"]
    endpoints = normalized.get("endpoints", [])
    test_cases: list[dict] = []
    test_points: list[dict] = []

    for endpoint in endpoints:
        cases = generate_endpoint_cases(endpoint, contract)
        test_cases.extend(cases)
        test_points.append({
            "接口中文名称": endpoint["summary"],
            "method": endpoint["method"],
            "path": endpoint["path"],
            "鉴权": "需要鉴权" if endpoint.get("auth", {}).get("requires_authorization") else "公共接口",
            "关键入参": summarize_key_fields(endpoint),
            "重点场景": [case["测试场景"] for case in cases],
        })

    return {
        "contract_title": normalized.get("title", ""),
        "contract_version": normalized.get("version", ""),
        "test_points": test_points,
        "test_cases": test_cases,
        "pending_items": [
            item for item in detect_pending_items(contract)
            if item.get("接口中文名称") == "全局" or item.get("接口中文名称") in {endpoint["summary"] for endpoint in endpoints}
        ],
    }


def scene_flags(scene: str) -> dict[str, bool]:
    text = scene or ""
    return {
        "positive": "正向" in text,
        "negative": any(token in text for token in ("异常", "非法", "缺少", "失败")),
        "boundary": any(token in text for token in ("边界", "等价类", "长度", "校验")),
        "flow": any(token in text for token in ("流程", "状态流转", "重复操作", "上游资源")),
    }


def parse_json_object(value: str, field_name: str, allow_empty: bool = False) -> tuple[Any, str | None]:
    if value in ("", None):
        if allow_empty:
            return "", None
        return None, f"{field_name} 为空"
    try:
        parsed = json.loads(value)
    except json.JSONDecodeError:
        return None, f"{field_name} 不是合法 JSON"
    return parsed, None


def match_endpoint(case_path: str, endpoints: list[dict]) -> dict | None:
    actual_path = case_path.split("?", 1)[0]
    candidates = []
    for endpoint in endpoints:
        if re.match(endpoint["path_regex"], actual_path):
            candidates.append(endpoint)
    if len(candidates) == 1:
        return candidates[0]
    if candidates:
        return candidates[0]
    return None


def extract_case_request_values(endpoint: dict, case_path: str) -> dict:
    parsed = urlparse(case_path)
    actual_path = parsed.path
    path_values = {}
    template_parts = endpoint["path"].strip("/").split("/")
    actual_parts = actual_path.strip("/").split("/")
    for template, actual in zip(template_parts, actual_parts):
        if template.startswith("{") and template.endswith("}"):
            path_values[template[1:-1]] = actual
    query_values = {key: values[0] if len(values) == 1 else values for key, values in parse_qs(parsed.query).items()}
    return {"path_values": path_values, "query_values": query_values}


def validate_value_against_field(value: Any, field: dict) -> list[str]:
    issues = []
    field_type = field.get("type", "string")
    if value is None:
        if field.get("required"):
            issues.append("缺少必填值")
        return issues

    if field_type == "file":
        text = str(value)
        exts = field.get("x-file-exts") or []
        if exts:
            if "." not in text or text.rsplit(".", 1)[-1].lower() not in {item.lower() for item in exts}:
                issues.append("文件类型不符合约束")
        return issues

    if field_type == "integer":
        if not re.fullmatch(r"-?\d+", str(value)):
            issues.append("类型应为 integer")
        else:
            numeric_value = int(value)
            if field.get("minimum") is not None and numeric_value < field["minimum"]:
                issues.append("小于最小值")
            if field.get("exclusiveMinimum") and field.get("minimum") is not None and numeric_value <= field["minimum"]:
                issues.append("未满足大于最小值")
            if field.get("maximum") is not None and numeric_value > field["maximum"]:
                issues.append("大于最大值")
    elif field_type == "number":
        try:
            numeric_value = float(value)
        except (TypeError, ValueError):
            issues.append("类型应为 number")
        else:
            if field.get("minimum") is not None and numeric_value < field["minimum"]:
                issues.append("小于最小值")
            if field.get("exclusiveMinimum") and field.get("minimum") is not None and numeric_value <= field["minimum"]:
                issues.append("未满足大于最小值")
            if field.get("maximum") is not None and numeric_value > field["maximum"]:
                issues.append("大于最大值")
    elif field_type == "boolean":
        if str(value).lower() not in {"true", "false"} and not isinstance(value, bool):
            issues.append("类型应为 boolean")
    else:
        if not isinstance(value, str):
            issues.append("类型应为 string")
            return issues
        text = str(value)
        if field.get("minLength") is not None and len(text) < field["minLength"]:
            issues.append("长度小于最小值")
        if field.get("maxLength") is not None and len(text) > field["maxLength"]:
            issues.append("长度大于最大值")
        if field.get("enum") and text not in {str(item) for item in field["enum"]}:
            issues.append("不在枚举范围内")
        if field.get("pattern") and not re.fullmatch(field["pattern"], text):
            issues.append("不满足格式约束")
    return issues


def get_nested_value(payload: Any, path: str) -> Any:
    if payload in ("", None):
        return None
    cursor = payload
    for part in path.split("."):
        if part.endswith("[0]"):
            key = part[:-3]
            if not isinstance(cursor, dict):
                return None
            cursor = cursor.get(key)
            if not isinstance(cursor, list) or not cursor:
                return None
            cursor = cursor[0]
        else:
            if not isinstance(cursor, dict):
                return None
            cursor = cursor.get(part)
        if cursor is None:
            return None
    return cursor


def validate_assertions(assertion_text: str) -> list[str]:
    issues = []
    if not assertion_text:
        return ["断言响应为空"]
    fragments = [item.strip() for item in assertion_text.split(";") if item.strip()]
    if not fragments:
        return ["断言响应为空"]
    for fragment in fragments:
        if fragment in {"返回成功", "提示正常", "处理正确"}:
            issues.append("断言过于空泛")
            continue
        if not any(pattern.match(fragment) for pattern in ASSERTION_PATTERNS):
            issues.append(f"断言格式不可执行: {fragment}")
    return issues


def add_problem(problems: list[dict], level: str, interface_name: str, scene: str, description: str, basis: str, suggestion: str, dimension: str, deduction: int) -> None:
    problems.append({
        "问题等级": level,
        "接口中文名称": interface_name,
        "测试场景": scene,
        "问题描述": description,
        "契约依据": basis,
        "修改建议": suggestion,
        "维度": dimension,
        "扣分": deduction,
    })


def build_review_report(contract: dict, cases: list[dict]) -> dict:
    endpoints = contract.get("normalized", {}).get("endpoints", [])
    problems: list[dict] = []
    coverage: dict[str, dict[str, bool]] = {}

    for case in cases:
        interface_name = case.get("接口中文名称", "")
        scene = case.get("测试场景", "")
        coverage.setdefault(interface_name, {"positive": False, "negative": False, "boundary": False, "flow": False})
        flags = scene_flags(scene)
        for key, value in flags.items():
            coverage[interface_name][key] = coverage[interface_name][key] or value

        for header in CASE_HEADERS:
            if header not in case:
                add_problem(problems, "一般", interface_name, scene, f"缺少模板字段 {header}", "固定模板要求字段齐全", "补齐模板字段。", "模板规范性", 1)

        endpoint = match_endpoint(case.get("path", ""), endpoints)
        if endpoint is None:
            add_problem(problems, "严重", interface_name, scene, "path 未在契约接口清单中找到", case.get("path", ""), "核对 path 是否与契约一致。", "契约一致性", 10)
            continue

        request_values = extract_case_request_values(endpoint, case.get("path", ""))
        headers, headers_error = parse_json_object(case.get("headers", ""), "headers")
        if headers_error:
            add_problem(problems, "一般", interface_name, scene, headers_error, "headers 必须是 JSON 对象字符串", "将 headers 改成合法 JSON 对象字符串。", "模板规范性", 1)
            headers = {}
        if headers is not None and headers != "" and not isinstance(headers, dict):
            add_problem(problems, "严重", interface_name, scene, "headers 不是 JSON 对象", "headers 模板要求为 JSON 对象字符串", "将 headers 改成对象结构。", "模板规范性", 1)

        allow_empty_body = endpoint["method"] == "GET" or (not endpoint.get("body") and not endpoint.get("form_data"))
        body, body_error = parse_json_object(case.get("body", ""), "body", allow_empty=allow_empty_body)
        if body_error and not endpoint["special_flags"]["upload"]:
            add_problem(problems, "一般", interface_name, scene, body_error, "body 必须是合法 JSON 或空字符串", "将 body 改成合法 JSON 或按 GET 场景置空。", "模板规范性", 1)
            body = ""

        if endpoint.get("auth", {}).get("requires_authorization") and flags["positive"]:
            if not isinstance(headers, dict) or "Authorization" not in headers:
                add_problem(problems, "严重", interface_name, scene, "正向场景缺少 Authorization", endpoint["auth"]["reason"], "补充 Authorization: Bearer ${token}。", "契约一致性", 10)
            if not isinstance(headers, dict) or "X-Lemonban-Media-Type" not in headers:
                add_problem(problems, "严重", interface_name, scene, "正向场景缺少 X-Lemonban-Media-Type", endpoint["auth"]["reason"], "补充正确的鉴权版本头。", "契约一致性", 8)

        for field in endpoint.get("path_params", []):
            value = request_values["path_values"].get(field["name"])
            issues = validate_value_against_field(value, field)
            if flags["positive"] and issues:
                add_problem(problems, "严重", interface_name, scene, f"path 参数 {field['name']} 不符合契约: {'/'.join(issues)}", endpoint["path"], "修正 path 参数实际值。", "契约一致性", 8)

        for field in endpoint.get("query_params", []):
            value = request_values["query_values"].get(field["name"])
            issues = validate_value_against_field(value, field)
            if flags["positive"] and issues:
                add_problem(problems, "严重", interface_name, scene, f"query 参数 {field['name']} 不符合契约: {'/'.join(issues)}", field.get("description", field["name"]), "修正 query 参数取值。", "契约一致性", 8)

        if endpoint.get("body"):
            if flags["positive"] and not isinstance(body, dict):
                add_problem(problems, "严重", interface_name, scene, "正向场景 body 为空或不是对象", endpoint["path"], "补充合法 JSON body。", "契约一致性", 10)
            if isinstance(body, dict):
                for field in endpoint.get("body_fields", []):
                    value = get_nested_value(body, field["path"])
                    issues = validate_value_against_field(value, field)
                    if flags["positive"] and issues:
                        add_problem(problems, "严重", interface_name, scene, f"body 字段 {field['path']} 不符合契约: {'/'.join(issues)}", field.get("description", field["path"]), "修正 body 字段取值。", "契约一致性", 8)
                top_level_schema = (endpoint["body"] or {}).get("schema") or {}
                defined_properties = set((top_level_schema.get("properties") or {}).keys())
                for key in body.keys():
                    if defined_properties and key not in defined_properties:
                        add_problem(problems, "严重", interface_name, scene, f"body 存在契约未声明字段 {key}", endpoint["path"], "删除幻觉字段。", "契约一致性", 10)

        if flags["negative"] or flags["boundary"]:
            violation_count = 0
            if endpoint.get("auth", {}).get("requires_authorization") and isinstance(headers, dict) and "Authorization" not in headers:
                violation_count += 1
            for field in endpoint.get("path_params", []):
                if validate_value_against_field(request_values["path_values"].get(field["name"]), field):
                    violation_count += 1
            for field in endpoint.get("query_params", []):
                if validate_value_against_field(request_values["query_values"].get(field["name"]), field):
                    violation_count += 1
            if isinstance(body, dict):
                for field in endpoint.get("body_fields", []):
                    if validate_value_against_field(get_nested_value(body, field["path"]), field):
                        violation_count += 1
                for field in endpoint.get("form_data", []):
                    if validate_value_against_field(body.get(field["name"]), field):
                        violation_count += 1
            if violation_count == 0 and not flags["flow"]:
                add_problem(problems, "一般", interface_name, scene, "异常/边界场景没有真正命中任何契约约束", endpoint["path"], "让该场景直接体现缺失必填、类型错误、边界越界或鉴权缺失。", "参数覆盖完整性", 4)

        assertion_issues = validate_assertions(case.get("断言响应", ""))
        for issue in assertion_issues:
            level = "严重" if "为空" in issue or "不可执行" in issue else "一般"
            deduction = 5 if level == "严重" else 2
            add_problem(problems, level, interface_name, scene, issue, "断言响应需要可执行、可观察", "改成 JSONPath/状态码形式断言。", "断言可执行性", deduction)

    endpoint_by_name = {item["summary"]: item for item in endpoints}
    for interface_name, stats in coverage.items():
        endpoint = endpoint_by_name.get(interface_name)
        if not stats["positive"]:
            add_problem(problems, "严重", interface_name, "", "缺少正向主场景", "每个接口至少需要 1 条正向主场景", "补充主流程成功场景。", "场景覆盖完整性", 8)
        if not stats["negative"]:
            add_problem(problems, "一般", interface_name, "", "缺少异常场景", "每个接口至少需要 1 条异常场景", "补充缺失必填、鉴权错误或非法参数场景。", "场景覆盖完整性", 5)
        if not stats["boundary"]:
            add_problem(problems, "一般", interface_name, "", "缺少边界/等价类场景", "每个接口至少需要 1 条单参数边界或等价类场景", "补充长度、范围、枚举或类型边界场景。", "场景覆盖完整性", 4)
        if endpoint and (endpoint["special_flags"]["state_flow"] or endpoint["dependencies"]) and not stats["flow"]:
            add_problem(problems, "一般", interface_name, "", "缺少流程/状态场景", "存在依赖链或状态流转特征", "补充上游资源不存在、重复操作或非法状态流转场景。", "流程/状态覆盖", 4)

    dimension_scores = []
    total_score = 0
    severe_exists = any(item["问题等级"] == "严重" for item in problems)
    for dimension, full_score in RUBRIC_DIMENSIONS.items():
        dimension_problems = [item for item in problems if item["维度"] == dimension]
        deduction = min(full_score, sum(item["扣分"] for item in dimension_problems))
        score = full_score - deduction
        total_score += score
        reasons = "；".join(unique_list([item["问题描述"] for item in dimension_problems])) if dimension_problems else "无"
        dimension_scores.append({
            "评分维度": dimension,
            "满分": full_score,
            "得分": score,
            "扣分原因": reasons,
        })

    if severe_exists or total_score < 70:
        conclusion = "不通过"
    elif total_score < 80:
        conclusion = "待补充后复审"
    elif total_score < 90:
        conclusion = "可用"
    else:
        conclusion = "优秀"

    coverage_rows = []
    for interface_name, stats in coverage.items():
        coverage_rows.append({
            "接口中文名称": interface_name,
            "是否有正向": "是" if stats["positive"] else "否",
            "是否有异常": "是" if stats["negative"] else "否",
            "是否有边界/等价类": "是" if stats["boundary"] else "否",
            "是否有流程/状态场景": "是" if stats["flow"] else "否",
            "结论": "通过" if all(stats.values()) or (stats["positive"] and stats["negative"] and stats["boundary"]) else "待补充",
        })

    return {
        "problems": problems,
        "coverage": coverage_rows,
        "dimension_scores": dimension_scores,
        "score": total_score,
        "conclusion": conclusion,
    }


def markdown_escape(value: Any) -> str:
    return str(value).replace("|", "\\|").replace("\n", "<br>")


def render_markdown_table(headers: list[str], rows: list[dict]) -> str:
    if not rows:
        return "| " + " | ".join(headers) + " |\n| " + " | ".join(["---"] * len(headers)) + " |\n"
    lines = [
        "| " + " | ".join(headers) + " |",
        "| " + " | ".join(["---"] * len(headers)) + " |",
    ]
    for row in rows:
        lines.append("| " + " | ".join(markdown_escape(row.get(header, "")) for header in headers) + " |")
    return "\n".join(lines)


def render_test_points_markdown(contract: dict, payload: dict) -> str:
    normalized = contract["normalized"]
    lines = [f"# {normalized.get('title') or '接口契约'}_接口测试要点", ""]
    lines.append("## 公共规则")
    common_rows = [{
        "项目": "基础信息",
        "内容": f"版本：{normalized.get('version', '') or '未提供'}；接口数量：{len(normalized.get('endpoints', []))}",
    }, {
        "项目": "Base URL",
        "内容": "；".join(normalized.get("common_rules", {}).get("base_urls", [])) or "契约未明确",
    }, {
        "项目": "通用响应",
        "内容": "、".join(normalized.get("common_rules", {}).get("response_fields", [])) or "契约未明确",
    }]
    for header_rule in normalized.get("common_rules", {}).get("headers", []):
        common_rows.append({
            "项目": header_rule["name"],
            "内容": f"{' / '.join(header_rule.get('allowed_values', []))}；{header_rule.get('rule', '')}",
        })
    lines.append(render_markdown_table(["项目", "内容"], common_rows))
    lines.append("")
    lines.append("## 接口测试设计")
    lines.append("")
    for item in payload.get("test_points", []):
        lines.append(f"### {item['接口中文名称']}")
        lines.append(f"- method/path：`{item['method']} {item['path']}`")
        lines.append(f"- 鉴权：{item['鉴权']}")
        lines.append(f"- 关键入参：{'；'.join(item['关键入参']) or '契约未给出明确入参约束'}")
        lines.append(f"- 重点场景：{'；'.join(item['重点场景'])}")
        lines.append("")
    return "\n".join(lines).strip() + "\n"


def render_pending_markdown(payload: dict) -> str:
    rows = payload.get("pending_items", [])
    lines = ["# 接口测试待确认项", ""]
    if not rows:
        lines.append("未识别到待确认项。")
        return "\n".join(lines) + "\n"
    lines.append(render_markdown_table(["接口中文名称", "原始契约描述", "不明确点", "影响", "待确认问题"], rows))
    return "\n".join(lines) + "\n"


def render_review_markdown(report: dict) -> str:
    lines = ["# 接口测试用例审核评分报告", ""]
    lines.append("## A. 问题清单")
    problem_rows = []
    for index, item in enumerate(report.get("problems", []), 1):
        row = {"序号": index}
        row.update({key: item.get(key, "") for key in ["问题等级", "接口中文名称", "测试场景", "问题描述", "契约依据", "修改建议"]})
        problem_rows.append(row)
    lines.append(render_markdown_table(["序号", "问题等级", "接口中文名称", "测试场景", "问题描述", "契约依据", "修改建议"], problem_rows))
    lines.append("")
    lines.append("## B. 覆盖率统计")
    lines.append(render_markdown_table(["接口中文名称", "是否有正向", "是否有异常", "是否有边界/等价类", "是否有流程/状态场景", "结论"], report.get("coverage", [])))
    lines.append("")
    lines.append("## C. 评分表")
    lines.append(render_markdown_table(["评分维度", "满分", "得分", "扣分原因"], report.get("dimension_scores", [])))
    lines.append("")
    lines.append("## D. 最终结论")
    lines.append("")
    lines.append(f"- 总分：{report.get('score', 0)}")
    lines.append(f"- 结论：{report.get('conclusion', '未判定')}")
    return "\n".join(lines) + "\n"


def autosize_columns(worksheet) -> None:
    for column_cells in worksheet.columns:
        max_length = 0
        column_letter = column_cells[0].column_letter
        for cell in column_cells:
            value = "" if cell.value is None else str(cell.value)
            max_length = max(max_length, len(value))
            cell.alignment = Alignment(vertical="top", wrap_text=True)
        worksheet.column_dimensions[column_letter].width = min(max(max_length + 2, 12), 50)


def write_cases_xlsx(rows: list[dict], output: Path) -> None:
    wb = Workbook()
    ws = wb.active
    ws.title = "接口测试用例"
    ws.append(CASE_HEADERS)
    for cell in ws[1]:
        cell.font = Font(bold=True)
        cell.alignment = Alignment(horizontal="center", vertical="center")
    for row in rows:
        ws.append([row.get(header, "") for header in CASE_HEADERS])
    autosize_columns(ws)
    wb.save(output)
