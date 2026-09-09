//! 扫描器与元数据校验（需求 §4.1–§4.3、§8.1）。
//!
//! 模块边界：扫描器只读文件（§10.2），不写、不删、不改。
//! - 只将源根直接子目录中的 SKILL.md 识别为 Skill（不把 references 中的示例当独立技能）
//! - 元数据校验：name/description 必填、名称格式与目录名一致、YAML 行号定位
//! - 内容摘要：逐文件 SHA-256，技能摘要由排序后的 (相对路径, 大小, 摘要) 计算
//! - 符号链接/目录联接 → 不支持；元数据引用外部目录 → 不支持

use crate::error::{AppError, ErrorCode};
use crate::windows_paths;
use globset::{Glob, GlobSet, GlobSetBuilder};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// 集合扫描默认跳过的目录名（§4.3）。`dist/` 等编译产物可能是运行资源，不排除。
const DEFAULT_SKIP_DIRS: &[&str] = &[
    ".git",
    ".svn",
    ".hg",
    "node_modules",
    ".venv",
    "__pycache__",
    ".skilldock-transactions",
    ".system",
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
    /// 相对技能目录的路径，使用 `/` 分隔
    pub rel_path: String,
    pub size: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationStatus {
    Valid,
    Invalid,
    Unsupported,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationIssue {
    pub code: String,
    pub message: String,
    pub line: Option<usize>,
    pub column: Option<usize>,
}

impl ValidationIssue {
    fn new(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.to_string(),
            message: message.into(),
            line: None,
            column: None,
        }
    }
    fn at(code: &str, message: impl Into<String>, line: usize, column: usize) -> Self {
        Self {
            code: code.to_string(),
            message: message.into(),
            line: Some(line),
            column: Some(column),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ScannedSkill {
    /// 相对源根的路径（如 `code-review`），`/` 分隔
    pub rel_path: String,
    pub dir_name: String,
    pub abs_path: PathBuf,
    pub name: Option<String>,
    pub description: Option<String>,
    pub file_count: u32,
    pub total_bytes: u64,
    pub last_content_changed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub digest: Option<String>,
    pub manifest: Vec<FileEntry>,
    /// 被源库忽略规则排除的文件（相对技能目录）
    pub excluded_by_ignore: Vec<String>,
    pub status: ValidationStatus,
    pub issues: Vec<ValidationIssue>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceCandidate {
    pub path: String,
    pub origin: String,
    pub valid_skill_count: u32,
    pub invalid_skill_count: u32,
}

// ---------------------------------------------------------------------------
// 摘要（§8.1）
// ---------------------------------------------------------------------------

pub fn sha256_file(path: &Path) -> Result<String, AppError> {
    let mut f = std::fs::File::open(path).map_err(AppError::from)?;
    let mut h = Sha256::new();
    std::io::copy(&mut f, &mut h).map_err(AppError::from)?;
    Ok(hex::encode(h.finalize()))
}

/// 技能摘要：排序后的 (相对路径, 大小, 文件摘要) 的 SHA-256（§8.1）。
pub fn digest_manifest(files: &[FileEntry]) -> String {
    let mut sorted: Vec<&FileEntry> = files.iter().collect();
    sorted.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
    let mut h = Sha256::new();
    for f in sorted {
        h.update(f.rel_path.as_bytes());
        h.update([0u8]);
        h.update(f.size.to_string().as_bytes());
        h.update([0u8]);
        h.update(f.sha256.as_bytes());
        h.update(b"\n");
    }
    hex::encode(h.finalize())
}

// ---------------------------------------------------------------------------
// SKILL.md 解析与校验（§4.2）
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct Frontmatter {
    name: Option<String>,
    description: Option<String>,
    external_refs: Vec<String>,
}

/// 解析 YAML 头：兼容 BOM，正文与其余元数据按原始内容保留（本函数只读）。
fn parse_frontmatter(raw: &[u8]) -> Result<Frontmatter, ValidationIssue> {
    let text = String::from_utf8_lossy(raw);
    let text = text.strip_prefix('\u{feff}').unwrap_or(&text);
    let mut lines = text.lines();
    match lines.next() {
        Some(l) if l.trim_end() == "---" => {}
        _ => {
            return Err(ValidationIssue::at(
                "yaml_error",
                "SKILL.md 缺少 YAML 头（第 1 行应为 `---`）",
                1,
                1,
            ))
        }
    }
    let mut yaml = String::new();
    let mut closed = false;
    let mut line_no = 1usize;
    for line in lines {
        line_no += 1;
        if line.trim_end() == "---" {
            closed = true;
            break;
        }
        yaml.push_str(line);
        yaml.push('\n');
    }
    if !closed {
        return Err(ValidationIssue::at(
            "yaml_error",
            "SKILL.md 的 YAML 头未闭合（缺少结束 `---` 行）",
            line_no.max(1),
            1,
        ));
    }
    let value: serde_yaml::Value = serde_yaml::from_str(&yaml).map_err(|e| {
        let (line, col) = e
            .location()
            .map(|l| (l.line() + 1, l.column()))
            .unwrap_or((2, 1));
        ValidationIssue::at("yaml_error", format!("SKILL.md YAML 格式错误：{e}"), line, col)
    })?;
    let mapping = value.as_mapping().ok_or_else(|| {
        ValidationIssue::at("yaml_error", "SKILL.md 的 YAML 头必须是键值映射", 2, 1)
    })?;
    let get_str = |key: &str| -> Option<String> {
        mapping
            .get(serde_yaml::Value::String(key.to_string()))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    };
    let mut external_refs = Vec::new();
    collect_external_refs(&value, &mut external_refs);
    Ok(Frontmatter {
        name: get_str("name"),
        description: get_str("description"),
        external_refs,
    })
}

/// 元数据字符串值中引用外部目录的相对路径（`../`），命中则标记不支持（§4.2）。
fn collect_external_refs(v: &serde_yaml::Value, out: &mut Vec<String>) {
    match v {
        serde_yaml::Value::String(s) => {
            if s.contains("../") || s.contains("..\\") {
                out.push(s.clone());
            }
        }
        serde_yaml::Value::Sequence(seq) => seq.iter().for_each(|x| collect_external_refs(x, out)),
        serde_yaml::Value::Mapping(m) => m.values().for_each(|x| collect_external_refs(x, out)),
        _ => {}
    }
}

/// 名称规则：1–64 个小写字母、数字或连字符；连字符不能连续或位于首尾（§4.2）。
fn validate_name(name: &str) -> Option<ValidationIssue> {
    let ok = !name.is_empty()
        && name.len() <= 64
        && name.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        && !name.starts_with('-')
        && !name.ends_with('-')
        && !name.contains("--");
    if ok {
        None
    } else {
        Some(ValidationIssue::new(
            "bad_name",
            format!("名称 `{name}` 不符合规范：1–64 个小写字母、数字或连字符，连字符不连续且不在首尾"),
        ))
    }
}

// ---------------------------------------------------------------------------
// 技能目录遍历
// ---------------------------------------------------------------------------

fn should_skip_dir(name: &str) -> bool {
    DEFAULT_SKIP_DIRS.iter().any(|s| s.eq_ignore_ascii_case(name))
}

/// 递归收集技能目录内的普通文件；命中重解析点记为不支持（§4.2/§8.5）。
fn collect_files(
    skill_dir: &Path,
    rel_prefix: &str,
    ignore: Option<&GlobSet>,
    root: &Path,
    files: &mut Vec<FileEntry>,
    excluded: &mut Vec<String>,
    issues: &mut Vec<ValidationIssue>,
) -> Result<Option<chrono::DateTime<chrono::Utc>>, AppError> {
    let mut latest: Option<chrono::DateTime<chrono::Utc>> = None;
    let mut entries: Vec<_> = std::fs::read_dir(skill_dir)
        .map_err(AppError::from)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(AppError::from)?;
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let name = entry.file_name().to_string_lossy().to_string();
        let meta = std::fs::symlink_metadata(entry.path()).map_err(AppError::from)?;
        if windows_paths::is_reparse_point(&meta) {
            issues.push(ValidationIssue::new(
                "symlink",
                format!("包含符号链接/目录联接，不支持分发：`{name}`"),
            ));
            continue;
        }
        let rel = if rel_prefix.is_empty() {
            name.clone()
        } else {
            format!("{rel_prefix}/{name}")
        };
        if meta.is_dir() {
            if should_skip_dir(&name) {
                continue;
            }
            if let Some(t) = collect_files(&entry.path(), &rel, ignore, root, files, excluded, issues)? {
                latest = Some(latest.map_or(t, |cur| cur.max(t)));
            }
            continue;
        }
        if !meta.is_file() {
            continue;
        }
        // 忽略规则：相对源根路径的 glob 语义（§4.3）
        let root_rel = root_rel_path(root, &entry.path());
        if ignore.is_some_and(|set| set.is_match(&root_rel)) {
            excluded.push(rel.clone());
            continue;
        }
        let sha = sha256_file(&entry.path())?;
        if let Ok(mtime) = meta.modified() {
            let t: chrono::DateTime<chrono::Utc> = mtime.into();
            latest = Some(latest.map_or(t, |cur| cur.max(t)));
        }
        files.push(FileEntry {
            rel_path: rel,
            size: meta.len(),
            sha256: sha,
        });
    }
    Ok(latest)
}

fn root_rel_path(root: &Path, p: &Path) -> String {
    p.strip_prefix(root)
        .unwrap_or(p)
        .to_string_lossy()
        .replace('\\', "/")
}

/// 扫描单个技能（执行器等逐技能路径）：不遍历源根其它目录。
pub fn scan_single_skill(
    source_root: &Path,
    rel_path: &str,
    ignore: Option<&GlobSet>,
) -> Result<Option<ScannedSkill>, AppError> {
    let dir = source_root.join(rel_path.replace('/', std::path::MAIN_SEPARATOR_STR));
    let meta = match std::fs::symlink_metadata(&dir) {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(AppError::from(e)),
    };
    let dir_name = dir
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    if !meta.is_dir() || windows_paths::is_reparse_point(&meta) || should_skip_dir(&dir_name) {
        return Ok(None);
    }
    if find_skill_md(&dir).is_none() {
        return Ok(None);
    }
    Ok(Some(scan_one_skill(source_root, &dir, rel_path, ignore)?))
}

// ---------------------------------------------------------------------------
// 扫描入口
// ---------------------------------------------------------------------------

pub struct ScanOptions<'a> {
    pub ignore: Option<&'a GlobSet>,
    /// 单技能模式：只扫描这些相对路径（§4.1 添加单个技能）
    pub skill_filter: Option<&'a [String]>,
    /// 每进入一个顶层技能目录时回调（scan.progress 分批推送用）
    pub on_dir: Option<&'a dyn Fn(&Path)>,
}

/// 扫描源根：直接子目录中含 SKILL.md 的识别为 Skill（结构 A/B 统一处理后的形态）。
/// 返回全部扫描到的技能（含无效项；无效项保留在列表并说明位置与原因）。
pub fn scan_source_root(source_root: &Path, opts: &ScanOptions) -> Result<Vec<ScannedSkill>, AppError> {
    if windows_paths::contains_reparse_point(source_root)? {
        return Err(AppError::new(
            ErrorCode::Unsupported,
            format!("源根路径包含符号链接/目录联接：{}", source_root.display()),
        ));
    }
    let mut skills = Vec::new();
    let mut entries: Vec<_> = std::fs::read_dir(source_root)
        .map_err(AppError::from)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(AppError::from)?;
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let dir_name = entry.file_name().to_string_lossy().to_string();
        let path = entry.path();
        let meta = std::fs::symlink_metadata(&path).map_err(AppError::from)?;
        if !meta.is_dir() || windows_paths::is_reparse_point(&meta) || should_skip_dir(&dir_name) {
            continue;
        }
        let rel = dir_name.clone();
        if let Some(filter) = opts.skill_filter {
            if !filter.iter().any(|f| f == &rel) {
                continue;
            }
        }
        let skill_md = find_skill_md(&path);
        if skill_md.is_none() {
            continue; // 普通集合说明目录/文件，不是 Skill（§4.1）
        }
        if let Some(cb) = opts.on_dir {
            cb(&path);
        }
        skills.push(scan_one_skill(source_root, &path, &rel, opts.ignore)?);
    }
    Ok(skills)
}

/// 目录中的 SKILL.md（Windows 大小写不敏感，按目录项实际名读取）。
fn find_skill_md(dir: &Path) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dir).ok()?;
    for e in entries.flatten() {
        if e.file_name().to_string_lossy().eq_ignore_ascii_case("SKILL.md") {
            return Some(e.path());
        }
    }
    None
}

fn scan_one_skill(
    source_root: &Path,
    dir: &Path,
    rel: &str,
    ignore: Option<&GlobSet>,
) -> Result<ScannedSkill, AppError> {
    let dir_name = dir
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let mut issues = Vec::new();
    let mut files = Vec::new();
    let mut excluded = Vec::new();
    let changed = collect_files(dir, "", ignore, source_root, &mut files, &mut excluded, &mut issues)?;
    let skill_md = find_skill_md(dir).expect("caller checked");

    let mut name = None;
    let mut description = None;
    match std::fs::read(&skill_md) {
        Ok(raw) => match parse_frontmatter(&raw) {
            Ok(fm) => {
                name = fm.name;
                description = fm.description;
                if !fm.external_refs.is_empty() {
                    issues.push(ValidationIssue::new(
                        "external_reference",
                        format!("元数据引用了技能目录外的相对路径：{}", fm.external_refs.join(", ")),
                    ));
                }
            }
            Err(issue) => issues.push(issue),
        },
        Err(e) => issues.push(ValidationIssue::new(
            "unreadable",
            format!("SKILL.md 无法完整读取：{e}"),
        )),
    }

    if let Some(n) = &name {
        if let Some(issue) = validate_name(n) {
            issues.push(issue);
        } else if !n.eq_ignore_ascii_case(&dir_name) {
            issues.push(ValidationIssue::new(
                "name_dir_mismatch",
                format!("名称 `{n}` 与目录名 `{dir_name}` 不一致"),
            ));
        }
    } else {
        issues.push(ValidationIssue::new("missing_name", "缺少必填字段 name"));
    }
    match &description {
        Some(d) if !d.is_empty() && d.chars().count() <= 1024 => {}
        Some(_) => issues.push(ValidationIssue::new(
            "missing_description",
            "description 需为 1–1024 字符",
        )),
        None => issues.push(ValidationIssue::new("missing_description", "缺少必填字段 description")),
    }

    let status = if issues
        .iter()
        .any(|i| i.code == "symlink" || i.code == "external_reference")
    {
        ValidationStatus::Unsupported
    } else if issues.is_empty() {
        ValidationStatus::Valid
    } else {
        ValidationStatus::Invalid
    };
    let digest = if status == ValidationStatus::Valid {
        Some(digest_manifest(&files))
    } else {
        None
    };
    Ok(ScannedSkill {
        rel_path: rel.to_string(),
        dir_name,
        abs_path: dir.to_path_buf(),
        name,
        description,
        file_count: files.len() as u32,
        total_bytes: files.iter().map(|f| f.size).sum(),
        last_content_changed_at: changed,
        digest,
        manifest: files,
        excluded_by_ignore: excluded,
        status,
        issues,
    })
}

// ---------------------------------------------------------------------------
// 候选源根发现（§4.1）
// ---------------------------------------------------------------------------

const CANDIDATE_DIRS: &[(&str, &str)] = &[
    ("skills", "skills"),
    (".agents/skills", "agents_skills"),
    (".claude/skills", "claude_skills"),
    (".codex/skills", "codex_skills"),
];

/// 对所选目录做浅层候选发现：根目录、`skills/`、`.agents/skills/`、`.claude/skills/`。
/// 只返回至少有一个 Skill 的候选；不递归、不合并不同集合。
pub fn discover_candidates(selected: &Path) -> Vec<SourceCandidate> {
    let mut out = Vec::new();
    let root_count = count_direct_skills(selected);
    if root_count > 0 {
        out.push(SourceCandidate {
            path: selected.to_string_lossy().to_string(),
            origin: "root".to_string(),
            valid_skill_count: root_count,
            invalid_skill_count: 0,
        });
    }
    for (sub, origin) in CANDIDATE_DIRS {
        let p = selected.join(sub);
        if !p.is_dir() {
            continue;
        }
        let n = count_direct_skills(&p);
        if n > 0 {
            out.push(SourceCandidate {
                path: p.to_string_lossy().to_string(),
                origin: origin.to_string(),
                valid_skill_count: n,
                invalid_skill_count: 0,
            });
        }
    }
    out
}

fn count_direct_skills(dir: &Path) -> u32 {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return 0;
    };
    let mut n = 0;
    for e in entries.flatten() {
        let p = e.path();
        let Ok(meta) = std::fs::symlink_metadata(&p) else {
            continue;
        };
        if meta.is_dir() && !should_skip_dir(&e.file_name().to_string_lossy()) && find_skill_md(&p).is_some() {
            n += 1;
        }
    }
    n
}

// ---------------------------------------------------------------------------
// 任意目录清单（目标端实时摘要；不应用源库忽略规则）
// ---------------------------------------------------------------------------

/// 计算一个目录的完整清单与摘要；目录不存在返回 None。
/// 命中重解析点返回 Unsupported（§8.5：提交前检查；含悬空链接——`exists()` 会误判其为缺席）。
pub fn digest_directory(dir: &Path) -> Result<Option<(String, Vec<FileEntry>)>, AppError> {
    // symlink_metadata 不跟随链接：悬空符号链接/目录联接也能被识别
    let meta = match std::fs::symlink_metadata(dir) {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(AppError::from(e)),
    };
    if windows_paths::is_reparse_point(&meta) {
        return Err(AppError::new(
            ErrorCode::Unsupported,
            format!(
                "目标是符号链接/目录联接，按规则不跟随、不改动；请确认后手动处理：{}",
                dir.display()
            ),
        ));
    }
    if !meta.is_dir() {
        return Err(AppError::invalid_path(format!(
            "目标位置已存在同名文件而非目录：{}",
            dir.display()
        )));
    }
    let mut files = Vec::new();
    let mut excluded = Vec::new();
    let mut issues = Vec::new();
    collect_files(dir, "", None, dir, &mut files, &mut excluded, &mut issues)?;
    if let Some(issue) = issues.iter().find(|i| i.code == "symlink") {
        return Err(AppError::new(
            ErrorCode::Unsupported,
            format!("目标目录包含符号链接/目录联接：{}", issue.message),
        ));
    }
    Ok(Some((digest_manifest(&files), files)))
}

/// 只校验 SKILL.md 元数据（不遍历文件；配合摘要缓存的快速路径）。
/// 返回 (name, description, issues, status)；嵌套符号链接由 digest 路径另行拒绝。
pub fn validate_metadata_only(
    dir: &Path,
    dir_name: &str,
) -> (Option<String>, Option<String>, Vec<ValidationIssue>, ValidationStatus) {
    let mut issues = Vec::new();
    let mut name = None;
    let mut description = None;
    match find_skill_md(dir) {
        Some(skill_md) => match std::fs::read(&skill_md) {
            Ok(raw) => match parse_frontmatter(&raw) {
                Ok(fm) => {
                    name = fm.name;
                    description = fm.description;
                    if !fm.external_refs.is_empty() {
                        issues.push(ValidationIssue::new(
                            "external_reference",
                            format!("元数据引用了技能目录外的相对路径：{}", fm.external_refs.join(", ")),
                        ));
                    }
                }
                Err(issue) => issues.push(issue),
            },
            Err(e) => issues.push(ValidationIssue::new(
                "unreadable",
                format!("SKILL.md 无法完整读取：{e}"),
            )),
        },
        None => issues.push(ValidationIssue::new(
            "unreadable",
            "缺少 SKILL.md",
        )),
    }
    if let Some(n) = &name {
        if let Some(issue) = validate_name(n) {
            issues.push(issue);
        } else if !n.eq_ignore_ascii_case(dir_name) {
            issues.push(ValidationIssue::new(
                "name_dir_mismatch",
                format!("名称 `{n}` 与目录名 `{dir_name}` 不一致"),
            ));
        }
    } else {
        issues.push(ValidationIssue::new("missing_name", "缺少必填字段 name"));
    }
    match &description {
        Some(d) if !d.is_empty() && d.chars().count() <= 1024 => {}
        Some(_) => issues.push(ValidationIssue::new(
            "missing_description",
            "description 需为 1–1024 字符",
        )),
        None => issues.push(ValidationIssue::new("missing_description", "缺少必填字段 description")),
    }
    let status = if issues
        .iter()
        .any(|i| i.code == "symlink" || i.code == "external_reference")
    {
        ValidationStatus::Unsupported
    } else if issues.is_empty() {
        ValidationStatus::Valid
    } else {
        ValidationStatus::Invalid
    };
    (name, description, issues, status)
}

// ---------------------------------------------------------------------------
// 摘要缓存（§8.1：修改时间仅作扫描加速提示，不作为提交判断依据）
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
struct DirSignature {
    files: u64,
    bytes: u64,
    max_mtime: Option<std::time::SystemTime>,
}

/// 只 stat 不读内容：递归 (文件数, 总字节, 最大 mtime)。跳过与 collect_files 相同的目录。
fn dir_signature(dir: &Path) -> Result<Option<DirSignature>, AppError> {
    fn walk(dir: &Path, sig: &mut DirSignature) -> Result<(), AppError> {
        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(e) => return Err(AppError::from(e)),
        };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            let meta = std::fs::symlink_metadata(entry.path()).map_err(AppError::from)?;
            if windows_paths::is_reparse_point(&meta) {
                continue; // 与 digest_directory 一样由调用方另行拒绝；签名层跳过
            }
            if meta.is_dir() {
                if !should_skip_dir(&name) {
                    walk(&entry.path(), sig)?;
                }
            } else if meta.is_file() {
                sig.files += 1;
                sig.bytes += meta.len();
                if let Ok(t) = meta.modified() {
                    sig.max_mtime = Some(sig.max_mtime.map_or(t, |cur: std::time::SystemTime| cur.max(t)));
                }
            }
        }
        Ok(())
    }
    if !dir.is_dir() {
        return Ok(None);
    }
    let mut sig = DirSignature { files: 0, bytes: 0, max_mtime: None };
    walk(dir, &mut sig)?;
    Ok(Some(sig))
}

struct DigestCacheEntry {
    signature: DirSignature,
    digest: String,
    manifest: Vec<FileEntry>,
}

static DIGEST_CACHE: std::sync::Mutex<Option<std::collections::HashMap<String, DigestCacheEntry>>> =
    std::sync::Mutex::new(None);

/// 带签名校验的目录摘要：签名（文件数+总字节+最大 mtime）未变则复用缓存，否则全新计算。
/// 仅用于矩阵/预览等展示路径；执行与提交复核一律用 `digest_directory` 全新计算。
pub fn digest_directory_cached(dir: &Path) -> Result<Option<(String, Vec<FileEntry>)>, AppError> {
    // 重解析点与非法形态必须每次都查（不缓存判断结果）
    let meta = match std::fs::symlink_metadata(dir) {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(AppError::from(e)),
    };
    if windows_paths::is_reparse_point(&meta) {
        return Err(AppError::new(
            ErrorCode::Unsupported,
            format!(
                "目标是符号链接/目录联接，按规则不跟随、不改动；请确认后手动处理：{}",
                dir.display()
            ),
        ));
    }
    if !meta.is_dir() {
        return Err(AppError::invalid_path(format!(
            "目标位置已存在同名文件而非目录：{}",
            dir.display()
        )));
    }
    let key = windows_paths::fold_case(dir);
    let Some(sig) = dir_signature(dir)? else {
        return Ok(None);
    };
    {
        let cache = DIGEST_CACHE.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(entry) = cache.as_ref().and_then(|c| c.get(&key)) {
            if entry.signature == sig {
                return Ok(Some((entry.digest.clone(), entry.manifest.clone())));
            }
        }
    }
    let Some((digest, manifest)) = digest_directory(dir)? else {
        return Ok(None);
    };
    let mut cache = DIGEST_CACHE.lock().unwrap_or_else(|e| e.into_inner());
    cache
        .get_or_insert_with(std::collections::HashMap::new)
        .insert(key, DigestCacheEntry { signature: sig, digest: digest.clone(), manifest: manifest.clone() });
    Ok(Some((digest, manifest)))
}

/// 清空摘要缓存（测试用）。
pub fn clear_digest_cache() {
    if let Some(c) = DIGEST_CACHE.lock().unwrap_or_else(|e| e.into_inner()).as_mut() {
        c.clear();
    }
}

// ---------------------------------------------------------------------------
// 忽略规则
// ---------------------------------------------------------------------------

pub fn build_globset(patterns: &[String]) -> Result<GlobSet, AppError> {
    let mut b = GlobSetBuilder::new();
    for p in patterns {
        let g = Glob::new(p).map_err(|e| {
            AppError::new(
                ErrorCode::ValidationFailed,
                format!("忽略规则 `{p}` 不是合法 glob：{e}"),
            )
        })?;
        b.add(g);
    }
    b.build().map_err(|e| AppError::internal(format!("忽略规则构建失败：{e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_rules() {
        assert!(validate_name("code-review").is_none());
        assert!(validate_name("a").is_none());
        assert!(validate_name("-bad").is_some());
        assert!(validate_name("bad-").is_some());
        assert!(validate_name("bad--name").is_some());
        assert!(validate_name("Bad").is_some());
        assert!(validate_name(&"x".repeat(65)).is_some());
    }

    #[test]
    fn digest_is_stable_and_order_independent() {
        let a = FileEntry { rel_path: "b.md".into(), size: 1, sha256: "aa".into() };
        let b = FileEntry { rel_path: "a.md".into(), size: 2, sha256: "bb".into() };
        let c = FileEntry { rel_path: "a.md".into(), size: 2, sha256: "cc".into() };
        let d1 = digest_manifest(&[a.clone(), b.clone()]);
        let d2 = digest_manifest(&[b.clone(), a.clone()]);
        assert_eq!(d1, d2);
        let d3 = digest_manifest(&[a, c]);
        assert_ne!(d1, d3);
    }

    #[test]
    fn frontmatter_parse_with_bom_and_line_numbers() {
        let good = "\u{feff}---\nname: demo\ndescription: 你好\n---\n# body\n";
        let fm = parse_frontmatter(good.as_bytes()).unwrap();
        assert_eq!(fm.name.as_deref(), Some("demo"));
        assert_eq!(fm.description.as_deref(), Some("你好"));

        let bad = "---\nname: [unclosed\ndescription: x\n---\n";
        let err = parse_frontmatter(bad.as_bytes()).unwrap_err();
        assert_eq!(err.code, "yaml_error");
        assert!(err.line.unwrap() >= 2, "应定位到出错行，实际 {:?}", err.line);
    }
}
