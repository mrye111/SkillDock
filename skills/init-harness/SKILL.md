---
name: init-harness
description: Initialize or sync a reusable pytest-playwright multi-agent Web UI test harness in a project. Use when the user wants to onboard a new Web project to the claude-web-test-harness pattern, generate or update harness files from harness.project.yaml, create a missing profile draft, clean demo TesterHome artifacts, or repeatedly synchronize managed harness blocks without overwriting custom project code.
---

# Init Harness

Use this skill to initialize or sync a per-project Web UI test harness. The target project keeps its own `harness.project.yaml`, `page_map/`, `data/`, `tests/`, and `artifacts/`; this global skill supplies the reusable initialization logic.

## Workflow

1. Run the script from the target project root in plan mode first:
   ```bash
   python C:/Users/23167/.agents/skills/init-harness/scripts/init_harness.py --project-root . --plan
   ```
2. If `harness.project.yaml` is missing, the script creates a minimal draft and stops. Ask the user to fill or confirm it before applying.
3. If the profile exists, read the plan summary. Do not apply destructive changes unless the user explicitly asked to apply.
4. Apply only after confirmation or when the user explicitly requests it:
   ```bash
   python C:/Users/23167/.agents/skills/init-harness/scripts/init_harness.py --project-root . --apply
   ```
5. After apply, tell the user what changed and what still needs manual attention.

## Design Rules

- One project gets one independent harness workspace.
- `harness.project.yaml` lives in the project root.
- The skill only initializes or syncs harness infrastructure; it does not start page exploration, test-case design, test-writing, or review.
- v1 is fixed to Python `pytest + pytest-playwright + Playwright`.
- Use managed blocks for repeatable updates and preserve custom blocks.
- Prefer plan/apply. Deletions are only for known TesterHome demo artifacts and only during apply.
- Existing unmanaged files are not overwritten; the script emits manual actions instead.

## Profile Shape

Use a minimal schema. Keep complex login flows in custom hooks instead of expanding YAML into a test DSL.

```yaml
project_name: my-shop
base_url: https://staging.example.com

paths:
  tests_dir: tests
  data_dir: data
  page_map_dir: page_map
  artifacts_dir: artifacts

runner:
  test_command: python -m pytest tests/ -v

auth:
  needs_auth: true
  auth_mode: basic
  login_url: /login
  username_env: E2E_USERNAME
  password_env: E2E_PASSWORD
  username_selector: 'input[name="email"]'
  password_selector: 'input[name="password"]'
  submit_selector: 'button[type="submit"]'
  post_login_url_pattern: /dashboard

project_contract:
  primary_flows:
    - 商品列表搜索
  notes:
    - 仅测试 staging
```

`auth.auth_mode` may be `basic`, `custom`, or `none`. For `custom`, the script creates a custom hook area in `conftest.py` and documents the expectation in `CLAUDE.md`.

## Bundled Resources

- `scripts/init_harness.py`: deterministic plan/apply implementation.
- `assets/templates/`: managed markdown templates for artifacts, Claude subagents, and rule guidance.
- `assets/profile.example.yaml`: example minimal project profile.
- `references/profile-schema.md`: profile fields and operational behavior.

Read `references/profile-schema.md` only when editing the schema or explaining profile fields in detail.
