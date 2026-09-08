//! Agent 目录适配器（需求 §5）。静态注册表 + 路径模板解析 + 可用性检测。
//!
//! 边界（§10.2）：适配器提供目录与格式能力，不包含复制引擎；
//! 不通过执行任意命令或启动 Agent 来检测安装状态（§5.2.2）。

use crate::contract::*;
use crate::error::{AppError, AppResult, ErrorCode};
use crate::windows_paths;
use std::path::{Path, PathBuf};

/// 适配器规则核实日期（§5：文档规则已核实 ≠ 真实客户端已验证；发版时锁定实测矩阵）。
pub const ADAPTER_VERSION: &str = "2026-09-08";

pub struct AdapterDef {
    pub id: &'static str,
    pub display_name: &'static str,
    pub docs_url: &'static str,
    pub scopes: &'static [TargetScope],
    /// 用户级模板；`%USERPROFILE%` 由后端解析
    pub user_template: Option<&'static str>,
    /// 项目级模板；`{PROJECT}` 由后端解析为目标项目根
    pub project_template: Option<&'static str>,
    /// 兼容读取提示（§5 共享读取）
    pub also_read_by: &'static [&'static str],
    pub notes: &'static [&'static str],
    /// 安装痕迹候选（存在即「发现了应用或配置线索」）
    pub detect_hints: &'static [&'static str],
    /// CLAUDE_CONFIG_DIR 等环境变量覆盖（仅 Claude Code 使用）
    pub env_override: Option<&'static str>,
}

pub static ADAPTERS: &[AdapterDef] = &[
    AdapterDef {
        id: "codex",
        display_name: "Codex",
        docs_url: "https://learn.chatgpt.com/docs/build-skills",
        scopes: &[TargetScope::User, TargetScope::Project],
        user_template: Some("%USERPROFILE%\\.agents\\skills"),
        project_template: Some("{PROJECT}\\.agents\\skills"),
        also_read_by: &["Cursor 也会读取 .agents 下的兼容技能目录"],
        notes: &[
            "旧路径 %USERPROFILE%\\.codex\\skills 需按当前版本确认，不自动双写或迁移",
            "项目发现可能向仓库根逐级查找",
        ],
        detect_hints: &["%USERPROFILE%\\.agents", "%USERPROFILE%\\.codex"],
        env_override: None,
    },
    AdapterDef {
        id: "claude_code",
        display_name: "Claude Code",
        docs_url: "https://code.claude.com/docs/en/skills",
        scopes: &[TargetScope::User, TargetScope::Project],
        user_template: Some("%USERPROFILE%\\.claude\\skills"),
        project_template: Some("{PROJECT}\\.claude\\skills"),
        also_read_by: &["Cursor 与 VS Code 也会读取 .claude 下的兼容技能目录"],
        notes: &["设置了 CLAUDE_CONFIG_DIR 时优先使用该配置目录"],
        detect_hints: &["%USERPROFILE%\\.claude"],
        env_override: Some("CLAUDE_CONFIG_DIR"),
    },
    AdapterDef {
        id: "cursor",
        display_name: "Cursor",
        docs_url: "https://prod.cursor.com/docs/skills",
        scopes: &[TargetScope::User, TargetScope::Project],
        user_template: Some("%USERPROFILE%\\.cursor\\skills"),
        project_template: Some("{PROJECT}\\.cursor\\skills"),
        also_read_by: &[],
        notes: &["Cursor 还会读取 .agents、.claude、.codex 下的兼容技能目录，可能存在重复发现"],
        detect_hints: &["%USERPROFILE%\\.cursor"],
        env_override: None,
    },
    AdapterDef {
        id: "copilot_vscode",
        display_name: "GitHub Copilot (VS Code)",
        docs_url: "https://code.visualstudio.com/docs/agent-customization/agent-skills",
        scopes: &[TargetScope::User, TargetScope::Project],
        user_template: Some("%USERPROFILE%\\.copilot\\skills"),
        project_template: Some("{PROJECT}\\.github\\skills"),
        also_read_by: &["VS Code 也支持用户/项目级 .claude 与 .agents 技能目录"],
        notes: &["首版验收限定 Windows 桌面 VS Code；Copilot CLI 与网页端不在已验证范围"],
        detect_hints: &["%USERPROFILE%\\.copilot", "%USERPROFILE%\\.vscode"],
        env_override: None,
    },
    AdapterDef {
        id: "custom",
        display_name: "自定义目录",
        docs_url: "",
        scopes: &[TargetScope::Custom],
        user_template: None,
        project_template: None,
        also_read_by: &[],
        notes: &["按文件夹结构复制，不承诺特定 Agent 会读取该目录"],
        detect_hints: &[],
        env_override: None,
    },
];

pub fn find_adapter(id: &str) -> AppResult<&'static AdapterDef> {
    ADAPTERS
        .iter()
        .find(|a| a.id == id)
        .ok_or_else(|| AppError::new(ErrorCode::ValidationFailed, format!("未知适配器：{id}")))
}

/// 解析模板中的环境变量与占位符；返回 (路径, 来源说明)。
fn resolve_template(template: &str, project_root: Option<&Path>) -> AppResult<(PathBuf, String)> {
    let mut s = template.to_string();
    let mut source_parts = vec!["预设模板".to_string()];
    if s.contains("%USERPROFILE%") {
        let home = std::env::var("USERPROFILE").map_err(|_| {
            AppError::new(ErrorCode::InvalidPath, "无法解析 %USERPROFILE% 环境变量")
        })?;
        s = s.replace("%USERPROFILE%", &home);
        source_parts.push("%USERPROFILE% 环境变量".to_string());
    }
    if s.contains("{PROJECT}") {
        let root = project_root.ok_or_else(|| {
            AppError::new(
                ErrorCode::ValidationFailed,
                "项目级目标需要先选择目标项目目录",
            )
        })?;
        s = s.replace("{PROJECT}", &root.to_string_lossy());
        source_parts.push("用户选择的目标项目".to_string());
    }
    Ok((PathBuf::from(s), source_parts.join(" + ")))
}

/// 解析适配器在给定作用域下的建议路径（含环境变量覆盖逻辑）。
pub fn resolve_adapter_path(
    def: &AdapterDef,
    scope: TargetScope,
    project_root: Option<&Path>,
    custom_path: Option<&str>,
) -> AppResult<(PathBuf, String, String)> {
    // Claude Code：CLAUDE_CONFIG_DIR 覆盖用户级配置目录（§5）
    if let (Some(env_var), TargetScope::User) = (def.env_override, scope) {
        if let Ok(dir) = std::env::var(env_var) {
            if !dir.trim().is_empty() {
                let p = PathBuf::from(dir).join("skills");
                return Ok((
                    p,
                    def.user_template.unwrap_or("").to_string(),
                    format!("{env_var} 环境变量覆盖"),
                ));
            }
        }
    }
    let template = match scope {
        TargetScope::User => def.user_template,
        TargetScope::Project => def.project_template,
        TargetScope::Custom => None,
    };
    match (template, custom_path) {
        (Some(t), _) => {
            let (p, src) = resolve_template(t, project_root)?;
            Ok((p, t.to_string(), src))
        }
        (None, Some(custom)) => Ok((
            PathBuf::from(custom),
            custom.to_string(),
            "用户自行选择的目录".to_string(),
        )),
        (None, None) => Err(AppError::new(
            ErrorCode::ValidationFailed,
            format!("适配器 {} 在作用域 {scope:?} 下需要提供目录", def.id),
        )),
    }
}

/// 目录可用性检测（§5.1）：存在/将创建/可写/无权限/路径失效/不支持位置。
/// 目录存在时用临时探针文件验证真实写能力，随后立即删除。
pub fn check_availability(path: &Path) -> (Availability, Option<String>) {
    if let Some(reason) = windows_paths::unsupported_location_reason(path) {
        return (Availability::UnsupportedLocation, Some(reason));
    }
    let s = path.to_string_lossy();
    if s.is_empty() || s.contains("..") {
        return (
            Availability::InvalidPath,
            Some("路径为空或包含越界组件".to_string()),
        );
    }
    if windows_paths::is_long_path(path) {
        return (
            Availability::InvalidPath,
            Some("路径超过 240 字符，建议缩短或移动目录".to_string()),
        );
    }
    if !path.exists() {
        return (
            Availability::WillCreate,
            Some("目录不存在，将在首次执行时创建".to_string()),
        );
    }
    if !path.is_dir() {
        return (
            Availability::InvalidPath,
            Some("该位置已存在同名文件，不是目录".to_string()),
        );
    }
    match windows_paths::contains_reparse_point(path) {
        Ok(true) => {
            return (
                Availability::UnsupportedLocation,
                Some("路径包含符号链接/目录联接，首版不支持".to_string()),
            )
        }
        Err(e) => return (Availability::InvalidPath, Some(format!("路径检查失败：{e}"))),
        _ => {}
    }
    let probe = path.join(format!(".skilldock-probe-{}", uuid::Uuid::new_v4()));
    match std::fs::File::create(&probe) {
        Ok(_) => {
            let _ = std::fs::remove_file(&probe);
            (Availability::Exists, Some("目录存在且可写".to_string()))
        }
        Err(e) => {
            let code = e.raw_os_error();
            if code == Some(5) {
                (
                    Availability::NoPermission,
                    Some("目录存在但无写入权限".to_string()),
                )
            } else {
                (
                    Availability::NoPermission,
                    Some(format!("目录写入探测失败：{e}")),
                )
            }
        }
    }
}

/// 检测安装痕迹（只证明有痕迹，不代表已验证加载；§5.1）。
fn detect_hints(def: &AdapterDef) -> Vec<String> {
    let mut out = Vec::new();
    for hint in def.detect_hints {
        if let Ok((p, _)) = resolve_template(hint, None) {
            if p.exists() {
                out.push(format!("发现线索：{} 存在", p.display()));
            }
        }
    }
    out
}

/// 供 `list_adapters` 命令组装卡片数据。
pub fn adapter_info(def: &AdapterDef, project_root: Option<&Path>) -> AdapterInfo {
    let resolve = |scope: TargetScope, template: Option<&'static str>| -> Option<ResolvedPath> {
        let _t = template?;
        let (p, _template, src) = resolve_adapter_path(def, scope, project_root, None).ok()?;
        let canon = windows_paths::canonicalize(&p).unwrap_or(p);
        let (availability, detail) = check_availability(&canon);
        Some(ResolvedPath {
            path: canon.to_string_lossy().to_string(),
            source: src,
            availability,
            detail,
        })
    };
    AdapterInfo {
        adapter_id: def.id.to_string(),
        display_name: def.display_name.to_string(),
        adapter_version: ADAPTER_VERSION.to_string(),
        verified_at: ADAPTER_VERSION.to_string(),
        docs_url: def.docs_url.to_string(),
        scopes: def.scopes.to_vec(),
        user_template: def.user_template.map(|s| s.to_string()),
        project_template: def.project_template.map(|s| s.to_string()),
        resolved: AdapterResolved {
            user: resolve(TargetScope::User, def.user_template),
            project: resolve(TargetScope::Project, def.project_template),
        },
        detected_hints: detect_hints(def),
        also_read_by: def.also_read_by.iter().map(|s| s.to_string()).collect(),
        notes: def.notes.iter().map(|s| s.to_string()).collect(),
    }
}

/// 「此位置也可能被 X 读取」提示（§5 共享读取与重复发现处理）。
pub fn shared_read_warnings(adapter_id: &str, canonical_path: &Path) -> Vec<String> {
    let mut warnings = Vec::new();
    let identity = windows_paths::fold_case(canonical_path);
    for def in ADAPTERS {
        if def.id == adapter_id {
            continue;
        }
        for template in [def.user_template, def.project_template].into_iter().flatten() {
            let probe = template
                .replace("{PROJECT}\\", "")
                .replace("{PROJECT}", "");
            // 仅比较已知固定前缀目录（如 .claude\skills、.agents\skills）
            let marker = probe
                .trim_start_matches("%USERPROFILE%\\")
                .replace('/', "\\")
                .to_lowercase();
            if !marker.is_empty() && !marker.contains('%') && identity.ends_with(&marker) {
                let readers: Vec<&str> = match def.id {
                    "cursor" => vec!["Cursor"],
                    "copilot_vscode" => vec!["VS Code (Copilot)"],
                    _ => vec![def.display_name],
                };
                for r in readers {
                    let w = format!("此位置也可能被 {r} 读取");
                    if !warnings.contains(&w) {
                        warnings.push(w);
                    }
                }
            }
        }
    }
    for note in find_adapter(adapter_id).map(|d| d.also_read_by).unwrap_or(&[]) {
        warnings.push(note.to_string());
    }
    warnings.sort();
    warnings.dedup();
    warnings
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_covers_p0_adapters() {
        for id in ["codex", "claude_code", "cursor", "copilot_vscode", "custom"] {
            assert!(find_adapter(id).is_ok(), "缺少适配器 {id}");
        }
        assert!(find_adapter("unknown").is_err());
    }

    #[test]
    fn resolves_user_template() {
        let def = find_adapter("claude_code").unwrap();
        // 测试环境可能设置了 CLAUDE_CONFIG_DIR；清除后走模板
        let saved = std::env::var("CLAUDE_CONFIG_DIR").ok();
        std::env::remove_var("CLAUDE_CONFIG_DIR");
        let (p, _t, src) = resolve_adapter_path(def, TargetScope::User, None, None).unwrap();
        if let Some(v) = saved {
            std::env::set_var("CLAUDE_CONFIG_DIR", v);
        }
        assert!(p.to_string_lossy().ends_with(".claude\\skills"));
        assert!(src.contains("预设模板"));
    }

    #[test]
    fn project_scope_requires_root() {
        let def = find_adapter("codex").unwrap();
        assert!(resolve_adapter_path(def, TargetScope::Project, None, None).is_err());
        let (p, _, _) = resolve_adapter_path(
            def,
            TargetScope::Project,
            Some(Path::new(r"C:\work\proj")),
            None,
        )
        .unwrap();
        assert!(p.to_string_lossy().ends_with(r".agents\skills"));
    }

    #[test]
    fn availability_for_missing_and_writable_dirs() {
        let tmp = tempfile::tempdir().unwrap();
        let missing = tmp.path().join("not-there");
        let (a, _) = check_availability(&missing);
        assert_eq!(a, Availability::WillCreate);
        let (a, _) = check_availability(tmp.path());
        assert_eq!(a, Availability::Exists);
    }
}
