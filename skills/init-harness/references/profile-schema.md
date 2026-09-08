# init-harness profile schema

`harness.project.yaml` is intentionally small. It configures initialization and synchronization, not business test behavior.

## Required fields

- `project_name`: Short project name used in generated docs.
- `base_url`: Test environment base URL.
- `paths.tests_dir`: Test code directory. Default: `tests`.
- `paths.data_dir`: Test data directory. Default: `data`.
- `paths.page_map_dir`: Page map directory. Default: `page_map`.
- `paths.artifacts_dir`: Agent artifact directory. Default: `artifacts`.
- `runner.test_command`: Command agents should use to run tests.
- `auth.needs_auth`: Boolean.
- `auth.auth_mode`: `basic`, `custom`, or `none`.

## Basic auth fields

Used when `auth.needs_auth: true` and `auth.auth_mode: basic`.

- `auth.login_url`
- `auth.username_env`
- `auth.password_env`
- `auth.username_selector`
- `auth.password_selector`
- `auth.submit_selector`
- `auth.post_login_url_pattern`

## Custom auth

Use `auth.auth_mode: custom` for SSO, OTP, tenant selection, captcha-bypassed staging flows, or any multi-step login. The script preserves a custom section in `conftest.py`; the project owner fills it.

## Managed files

The script updates managed blocks in:

- `CLAUDE.md`
- `conftest.py`
- `.gitignore`
- `requirements.txt`
- `rule.md`

The script creates or updates managed static files in:

- `artifacts/README.md`
- `artifacts/agents/*.md`
- `.claude/agents/*.md`

`pytest.ini` is only overwritten when missing or already marked as managed.
