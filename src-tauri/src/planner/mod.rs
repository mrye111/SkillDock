//! 差异与冲突计划器（需求 §7.2、§8.1–§8.3）。
//!
//! 模块边界（§10.2）：计划器只生成操作，不执行写入。
//! 计划保存生成时的源/目标摘要与配置指纹；执行前逐项复核，变化即失效（§7.2）。

use crate::contract::*;
use crate::error::{AppError, AppResult, ErrorCode};
use crate::scanner::{self, FileEntry};
use crate::storage::{PlanGroupMeta, PlanRow, Store};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::collections::HashMap;

/// 内容三面（§8.2）：S 当前源、T 当前目标、B 上次托管基线；∅ 表示不存在。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Content<'a> {
    Absent,
    Present(&'a str),
}

#[derive(Debug)]
struct Decision {
    state: MatrixCellState,
    action: PlanAction,
    selected: bool,
    needs_backup: bool,
    conflict: Option<ConflictInfo>,
    blocked_reason: Option<String>,
}

impl Decision {
    fn simple(state: MatrixCellState, action: PlanAction, selected: bool) -> Self {
        Self {
            state,
            action,
            selected,
            needs_backup: false,
            conflict: None,
            blocked_reason: None,
        }
    }
    fn conflict(state: MatrixCellState, kind: &str, message: String, choices: Vec<ConflictChoice>) -> Self {
        Self {
            state,
            action: PlanAction::Skip,
            selected: false,
            needs_backup: false,
            conflict: Some(ConflictInfo {
                kind: kind.to_string(),
                message,
                available_choices: choices,
            }),
            blocked_reason: None,
        }
    }
}

/// §8.2 决策表。判定顺序：归属冲突 → 校验/暂停 → 源/目标缺失 → 非托管目录 → 普通内容比较。
fn decide(
    managed: bool,
    source: Content,
    target: Content,
    baseline: Content,
    validation: ValidationStatus,
    paused: bool,
    ownership_conflict: bool,
) -> Decision {
    if ownership_conflict {
        return Decision::conflict(
            MatrixCellState::OwnershipConflict,
            "ownership",
            "该目标目录已归属另一个技能库；禁止自动抢占，需明确转移归属".into(),
            vec![ConflictChoice::TransferOwnership, ConflictChoice::KeepTarget],
        );
    }
    if validation == ValidationStatus::Invalid {
        return Decision {
            state: MatrixCellState::Invalid,
            blocked_reason: Some("技能元数据校验失败，禁止同步".into()),
            ..Decision::simple(MatrixCellState::Invalid, PlanAction::Blocked, false)
        };
    }
    if validation == ValidationStatus::Unsupported {
        return Decision {
            state: MatrixCellState::Unsupported,
            blocked_reason: Some("技能包含不支持的内容（符号链接或外部引用）".into()),
            ..Decision::simple(MatrixCellState::Unsupported, PlanAction::Blocked, false)
        };
    }
    if paused {
        return Decision::simple(MatrixCellState::Paused, PlanAction::Skip, false);
    }

    match (managed, source, target, baseline) {
        // 无托管记录
        (false, Content::Present(_), Content::Absent, _) => {
            Decision::simple(MatrixCellState::ToAdd, PlanAction::Create, true)
        }
        (false, Content::Present(s), Content::Present(t), _)
            if digest_eq(Some(s), Some(t)) =>
        {
            Decision::conflict(
                MatrixCellState::SameContentUnmanaged,
                "same_content",
                "目标已存在相同内容的同名目录；可显式接管（只建立基线，不复制）".into(),
                vec![ConflictChoice::AdoptExisting, ConflictChoice::KeepTarget],
            )
        }
        (false, Content::Present(_), Content::Present(_), _) => Decision::conflict(
            MatrixCellState::UnmanagedConflict,
            "unmanaged_same_name",
            "目标已存在不同内容的非托管同名目录；默认保留，显式接管后才可覆盖".into(),
            vec![ConflictChoice::KeepTarget, ConflictChoice::TakeOver],
        ),
        (false, Content::Absent, _, _) => Decision {
            blocked_reason: None,
            ..Decision::simple(MatrixCellState::SourceRemoved, PlanAction::Skip, false)
        },

        // 有基线（含「不存在」标记）
        (true, Content::Absent, Content::Present(_), _) => {
            // 源已移除：默认保留目标，等待处理；整项移除走独立 remove 计划（§6.4/§8.3）
            Decision::simple(MatrixCellState::SourceRemoved, PlanAction::Skip, false)
        }
        (true, Content::Absent, Content::Absent, _) => {
            Decision::simple(MatrixCellState::SourceRemoved, PlanAction::Skip, false)
        }
        (true, Content::Present(s), Content::Absent, _) => {
            // 目标已删除：默认保留删除状态；手动选择重新安装
            let _ = s;
            Decision::conflict(
                MatrixCellState::TargetDeleted,
                "target_deleted",
                "目标目录已被删除；默认保留删除状态，可手动重新安装".into(),
                vec![ConflictChoice::KeepDeleted, ConflictChoice::Reinstall],
            )
        }
        (true, Content::Present(s), Content::Present(t), baseline) => {
            let b = match baseline {
                Content::Present(b) => Some(b),
                Content::Absent => None,
            };
            let s_eq_b = digest_eq(Some(s), b);
            let t_eq_b = digest_eq(Some(t), b);
            let s_eq_t = digest_eq(Some(s), Some(t));
            match (s_eq_b, t_eq_b, s_eq_t) {
                (true, true, _) => Decision::simple(MatrixCellState::Synced, PlanAction::Skip, false),
                (false, true, _) => Decision {
                    needs_backup: true,
                    ..Decision::simple(MatrixCellState::SourceUpdated, PlanAction::Update, true)
                },
                (true, false, _) => Decision::conflict(
                    MatrixCellState::TargetModified,
                    "target_modified",
                    "目标端有本地修改；默认保留目标，查看差异后选择保留或覆盖".into(),
                    vec![
                        ConflictChoice::KeepTarget,
                        ConflictChoice::OverwriteWithSource,
                        ConflictChoice::PauseMapping,
                    ],
                ),
                (false, false, false) => Decision::conflict(
                    MatrixCellState::BothModified,
                    "both_modified",
                    "源与目标均已修改；默认保留目标，查看差异后选择保留或覆盖".into(),
                    vec![
                        ConflictChoice::KeepTarget,
                        ConflictChoice::OverwriteWithSource,
                        ConflictChoice::PauseMapping,
                    ],
                ),
                (false, false, true) => {
                    // 双方内容已一致：不复制，核验后刷新基线（§8.2）
                    Decision::simple(MatrixCellState::AlignedExternally, PlanAction::Adopt, true)
                }
            }
        }
    }
}

fn digest_eq(a: Option<&str>, b: Option<&str>) -> bool {
    match (a, b) {
        (Some(x), Some(y)) => x == y,
        (None, None) => true,
        _ => false,
    }
}

// ---------------------------------------------------------------------------
// 文件级差异
// ---------------------------------------------------------------------------

fn diff_manifests(
    source: &[FileEntry],
    target: &[FileEntry],
    source_dir: Option<&Path>,
) -> Vec<FileChange> {
    let mut changes = Vec::new();
    let target_map: HashMap<&str, &FileEntry> =
        target.iter().map(|f| (f.rel_path.as_str(), f)).collect();
    let source_map: HashMap<&str, &FileEntry> =
        source.iter().map(|f| (f.rel_path.as_str(), f)).collect();
    for s in source {
        match target_map.get(s.rel_path.as_str()) {
            None => changes.push(FileChange {
                kind: "add".into(),
                rel_path: s.rel_path.clone(),
                bytes: s.size,
                source_digest: Some(s.sha256.clone()),
                target_digest: None,
                diff_previewable: source_dir
                    .map(|d| previewable(d, &s.rel_path, s.size))
                    .unwrap_or(false),
            }),
            Some(t) if t.sha256 != s.sha256 => changes.push(FileChange {
                kind: "update".into(),
                rel_path: s.rel_path.clone(),
                bytes: s.size,
                source_digest: Some(s.sha256.clone()),
                target_digest: Some(t.sha256.clone()),
                diff_previewable: source_dir
                    .map(|d| previewable(d, &s.rel_path, s.size))
                    .unwrap_or(false),
            }),
            Some(_) => {}
        }
    }
    for t in target {
        if !source_map.contains_key(t.rel_path.as_str()) {
            changes.push(FileChange {
                kind: "delete".into(),
                rel_path: t.rel_path.clone(),
                bytes: t.size,
                source_digest: None,
                target_digest: Some(t.sha256.clone()),
                diff_previewable: false,
            });
        }
    }
    changes.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
    changes
}

/// §7.2：超过 1 MiB、超过 20,000 行、非 UTF-8 或为二进制 → 不做行内差异预览。
fn previewable(source_dir: &Path, rel: &str, size: u64) -> bool {
    if size > 1024 * 1024 {
        return false;
    }
    let path = source_dir.join(rel.replace('/', std::path::MAIN_SEPARATOR_STR));
    let Ok(bytes) = std::fs::read(&path) else {
        return false;
    };
    if bytes.contains(&0) {
        return false;
    }
    let Ok(text) = std::str::from_utf8(&bytes) else {
        return false;
    };
    text.lines().count() <= 20_000
}

/// 矩阵单元格：S 用最近一次扫描的库内摘要，T 实时计算（漂移检测必须真实；§2.2）。
/// 只产出已配置映射的单元格；未映射的技能×目标由前端渲染为「未选择」。
pub fn matrix_cells(
    store: &Store,
    library: &crate::storage::LibraryRow,
    skills: &[crate::storage::SkillRow],
) -> AppResult<HashMap<String, HashMap<String, MatrixCell>>> {
    let mut out: HashMap<String, HashMap<String, MatrixCell>> = HashMap::new();
    let mappings = store.mappings_for_library(&library.id)?;
    let mappings_by_skill: HashMap<String, Vec<&crate::storage::MappingRow>> =
        mappings.iter().fold(HashMap::new(), |mut acc, m| {
            acc.entry(m.skill_id.clone()).or_default().push(m);
            acc
        });
    for skill in skills {
        let Some(skill_mappings) = mappings_by_skill.get(&skill.id) else {
            continue;
        };
        let cells = out.entry(skill.id.clone()).or_default();
        for mapping in skill_mappings {
            // 已停用（取消勾选）的映射不产出单元格 → 前端显示「未选择」（§7.1/AC-14）
            if !mapping.enabled {
                continue;
            }
            let physical = store.get_physical_target(&mapping.physical_target_id)?;
            let target_dir = PathBuf::from(&physical.canonical_path).join(&mapping.target_dir_name);
            // 目标读取失败（符号链接等）：单元格标记不支持，不拖垮整个矩阵
            let target_scan = scanner::digest_directory_cached(&target_dir);
            if let Err(e) = target_scan {
                cells.insert(
                    mapping.physical_target_id.clone(),
                    MatrixCell {
                        state: MatrixCellState::Unsupported,
                        mapping_id: Some(mapping.id.clone()),
                        target_dir_name: Some(mapping.target_dir_name.clone()),
                        detail: Some(e.message),
                    },
                );
                continue;
            }
            let t_digest = target_scan.unwrap().map(|(d, _)| d);
            let baseline = store.get_baseline(&mapping.id)?;
            let managed = baseline.is_some();
            let b_digest = baseline.as_ref().and_then(|b| b.digest.clone());
            let source = if skill.missing {
                Content::Absent
            } else {
                match &skill.digest {
                    Some(d) => Content::Present(d),
                    None => Content::Absent,
                }
            };
            let target = match &t_digest {
                Some(d) => Content::Present(d),
                None => Content::Absent,
            };
            let baseline_c = match b_digest.as_deref() {
                Some(d) => Content::Present(d),
                None => Content::Absent,
            };
            let decision = decide(
                managed,
                source,
                target,
                baseline_c,
                skill.validation_status,
                mapping.paused_reason.is_some(),
                false,
            );
            cells.insert(
                mapping.physical_target_id.clone(),
                MatrixCell {
                    state: decision.state,
                    mapping_id: Some(mapping.id.clone()),
                    target_dir_name: Some(mapping.target_dir_name.clone()),
                    detail: decision
                        .conflict
                        .map(|c| c.message)
                        .or(decision.blocked_reason),
                },
            );
        }
    }
    Ok(out)
}

/// 恢复计划等外部模块复用的清单差异（恢复预览不要求源目录可预览差异）。
pub fn diff_manifests_pub(source: &[FileEntry], target: &[FileEntry]) -> Vec<FileChange> {
    diff_manifests(source, target, None)
}

// ---------------------------------------------------------------------------
// 计划生成
// ---------------------------------------------------------------------------

/// 生成同步/移除计划（§7.2）。读取当前磁盘实时状态，不依赖扫描缓存。
pub fn create_plan(store: &Store, input: &CreateSyncPlanInput) -> AppResult<SyncPlan> {
    let library = store.get_library(&input.library_id)?;
    if library.archived {
        return Err(AppError::new(
            ErrorCode::ValidationFailed,
            "该技能库已被移除记录，不能生成计划",
        ));
    }
    let source_root = library.source_root.clone().ok_or_else(|| {
        AppError::new(
            ErrorCode::ValidationFailed,
            "该技能库存在多个候选源根，请先选定源根目录",
        )
    })?;
    let source_root = PathBuf::from(&source_root);
    let ignore = scanner::build_globset(&library.ignore_patterns)?;

    let mut items: Vec<PlanItem> = Vec::new();
    let mut groups: HashMap<String, PlanGroupMeta> = HashMap::new();

    for mapping_id in &input.mapping_ids {
        let mapping = store.get_mapping(mapping_id)?;
        if mapping.library_id != library.id {
            return Err(AppError::new(
                ErrorCode::ValidationFailed,
                format!("映射 {mapping_id} 不属于技能库 {}", library.display_name),
            ));
        }
        let physical = store.get_physical_target(&mapping.physical_target_id)?;
        let target_dir = PathBuf::from(&physical.canonical_path).join(&mapping.target_dir_name);
        groups
            .entry(physical.id.clone())
            .or_insert_with(|| PlanGroupMeta {
                physical_target_id: physical.id.clone(),
                target_path: physical.canonical_path.clone(),
                shared_with_tools: store
                    .targets_on_physical(&physical.id)
                    .map(|ts| ts.iter().map(|t| t.display_name.clone()).collect())
                    .unwrap_or_default(),
            });

        let item = match input.operation {
            PlanOperation::Sync => {
                plan_sync_item(store, &library, &mapping, &source_root, &ignore, &target_dir)?
            }
            PlanOperation::Remove => {
                plan_remove_item(store, &library, &mapping, &target_dir)?
            }
            PlanOperation::Restore => {
                return Err(AppError::new(
                    ErrorCode::ValidationFailed,
                    "恢复计划请使用 create_restore_plan",
                ))
            }
        };
        items.push(item);
    }

    let fingerprint = compute_fingerprint(library.config_version, input.operation, &items);
    let plan_id = uuid::Uuid::new_v4().to_string();
    let groups_meta: Vec<PlanGroupMeta> = groups.into_values().collect();
    store.insert_plan(
        &plan_id,
        &library.id,
        input.operation,
        &fingerprint,
        &items,
        &groups_meta,
        library.config_version,
    )?;
    let row = store.get_plan(&plan_id)?;
    store.plan_view(&row)
}

/// 源技能实时内容：内容摘要走签名缓存（§8.1 加速提示），忽略规则每次现算，
/// SKILL.md 元数据每次全新校验（单文件，廉价）。嵌套链接 → Unsupported 而非计划失败。
fn live_source(
    skill: &crate::storage::SkillRow,
    source_root: &Path,
    ignore: &globset::GlobSet,
) -> AppResult<(Option<String>, Vec<FileEntry>, ValidationStatus, Vec<String>)> {
    let skill_dir = source_root.join(skill.rel_path.replace('/', std::path::MAIN_SEPARATOR_STR));
    if !skill_dir.is_dir() {
        return Ok((None, Vec::new(), skill.validation_status, Vec::new()));
    }
    let dir_name = skill
        .rel_path
        .rsplit('/')
        .next()
        .unwrap_or(&skill.rel_path)
        .to_string();
    let (_, _, _, status) = scanner::validate_metadata_only(&skill_dir, &dir_name);
    let raw = match scanner::digest_directory_cached(&skill_dir) {
        Ok(Some((d, m))) => Some((d, m)),
        Ok(None) => None,
        Err(e) if e.code == ErrorCode::Unsupported => {
            return Ok((None, Vec::new(), ValidationStatus::Unsupported, Vec::new()))
        }
        Err(e) => return Err(e),
    };
    let Some((_, raw_manifest)) = raw else {
        return Ok((None, Vec::new(), status, Vec::new()));
    };
    // 应用忽略规则（相对源根的 glob 语义；§4.3）
    let mut manifest = Vec::with_capacity(raw_manifest.len());
    let mut excluded = Vec::new();
    for f in raw_manifest {
        let root_rel = format!("{}/{}", skill.rel_path, f.rel_path);
        if ignore.is_match(&root_rel) {
            excluded.push(f.rel_path);
        } else {
            manifest.push(f);
        }
    }
    let digest = if status == ValidationStatus::Valid {
        Some(scanner::digest_manifest(&manifest))
    } else {
        None
    };
    Ok((digest, manifest, status, excluded))
}

fn plan_sync_item(
    store: &Store,
    _library: &crate::storage::LibraryRow,
    mapping: &crate::storage::MappingRow,
    source_root: &Path,
    ignore: &globset::GlobSet,
    target_dir: &Path,
) -> AppResult<PlanItem> {
    let skill = store.get_skill(&mapping.skill_id)?;
    let (s_digest, s_manifest, live_status, excluded) =
        live_source(&skill, source_root, ignore)?;
    // 目标端读取失败（如符号链接/目录联接）→ 单项阻塞，不拖垮整个预览（§7.2）
    let (t_digest, t_manifest, target_blocked) = match scanner::digest_directory_cached(target_dir) {
        Ok(Some((d, m))) => (Some(d), m, None),
        Ok(None) => (None, Vec::new(), None),
        Err(e) => (None, Vec::new(), Some(e.message)),
    };
    let baseline = store.get_baseline(&mapping.id)?;
    let managed = baseline.is_some();
    let b_digest: Option<&str> = baseline.as_ref().and_then(|b| b.digest.as_deref());
    // 目标目录名被其它库的活动映射占用 → 归属冲突（§8.2 末行）
    let ownership_conflict =
        match store.find_active_mapping(&mapping.physical_target_id, &mapping.target_dir_name)? {
            Some(other) => other.library_id != mapping.library_id || !mapping.enabled,
            None => false,
        };
    let paused = mapping.paused_reason.is_some() || !mapping.enabled;

    let source = match &s_digest {
        Some(d) => Content::Present(d),
        None => Content::Absent,
    };
    let target = match &t_digest {
        Some(d) => Content::Present(d),
        None => Content::Absent,
    };
    let baseline_c = match b_digest {
        Some(d) => Content::Present(d),
        None => Content::Absent,
    };
    let decision = match &target_blocked {
        Some(reason) => Decision {
            state: MatrixCellState::Unsupported,
            blocked_reason: Some(reason.clone()),
            ..Decision::simple(MatrixCellState::Unsupported, PlanAction::Blocked, false)
        },
        None => decide(
            managed,
            source,
            target,
            baseline_c,
            live_status,
            paused,
            ownership_conflict,
        ),
    };

    let file_changes = match decision.action {
        PlanAction::Create | PlanAction::Reinstall => {
            diff_manifests(&s_manifest, &[], Some(&source_root.join(&skill.rel_path)))
        }
        PlanAction::Update => {
            diff_manifests(&s_manifest, &t_manifest, Some(&source_root.join(&skill.rel_path)))
        }
        _ => Vec::new(),
    };
    // 文件删除只在「更新」中由清单差异表达（§8.1）；diff_manifests 已覆盖。
    let skill_name = skill
        .name
        .clone()
        .unwrap_or_else(|| mapping.target_dir_name.clone());
    Ok(PlanItem {
        item_id: uuid::Uuid::new_v4().to_string(),
        mapping_id: mapping.id.clone(),
        skill_id: skill.id.clone(),
        skill_name,
        rel_path: skill.rel_path.clone(),
        action: decision.action,
        file_changes,
        excluded_by_ignore: excluded
            .into_iter()
            .map(|rel_path| ExcludedFile { rel_path })
            .collect(),
        needs_backup: decision.needs_backup,
        target_path: target_dir.to_string_lossy().to_string(),
        state_before: decision.state,
        conflict: decision.conflict,
        blocked_reason: decision.blocked_reason,
        selected: decision.selected,
        decision: None,
        source_digest: s_digest,
        target_digest: t_digest,
        baseline_digest: baseline.map(|b| b.digest),
        restore: None,
    })
}

fn plan_remove_item(
    store: &Store,
    _library: &crate::storage::LibraryRow,
    mapping: &crate::storage::MappingRow,
    target_dir: &Path,
) -> AppResult<PlanItem> {
    let skill = store.get_skill(&mapping.skill_id)?;
    let baseline = store.get_baseline(&mapping.id)?;
    // 目标读取失败（符号链接等）→ 阻塞移除，不在未知内容上动手
    let (t_digest, t_manifest, read_blocked) = match scanner::digest_directory_cached(target_dir) {
        Ok(Some((d, m))) => (Some(d), m, None),
        Ok(None) => (None, Vec::new(), None),
        Err(e) => (None, Vec::new(), Some(e.message)),
    };
    if let Some(reason) = read_blocked {
        return Ok(PlanItem {
            item_id: uuid::Uuid::new_v4().to_string(),
            mapping_id: mapping.id.clone(),
            skill_id: skill.id.clone(),
            skill_name: skill.name.clone().unwrap_or_else(|| mapping.target_dir_name.clone()),
            rel_path: skill.rel_path.clone(),
            action: PlanAction::Blocked,
            file_changes: Vec::new(),
            excluded_by_ignore: Vec::new(),
            needs_backup: false,
            target_path: target_dir.to_string_lossy().to_string(),
            state_before: MatrixCellState::Unsupported,
            conflict: None,
            blocked_reason: Some(reason),
            selected: false,
            decision: None,
            source_digest: None,
            target_digest: None,
            baseline_digest: baseline.map(|b| b.digest),
            restore: None,
        });
    }

    let (action, selected, needs_backup, conflict, blocked_reason) = match (&baseline, &t_digest) {
        (None, _) => (
            PlanAction::Blocked,
            false,
            false,
            None,
            Some("该目录从未被本应用托管，不提供移除（§8.3）".to_string()),
        ),
        (Some(_b), None) => (
            // 目标已不存在：仅清理记录
            PlanAction::Remove,
            true,
            false,
            None,
            None,
        ),
        (Some(b), Some(t)) if digest_eq(b.digest.as_deref(), Some(t)) => {
            (PlanAction::Remove, true, true, None, None)
        }
        (Some(_), Some(_)) => (
            PlanAction::Skip,
            false,
            false,
            Some(ConflictInfo {
                kind: "target_modified".to_string(),
                message: "目标内容已被修改；看过差异并确认后才可备份当前内容后移除".to_string(),
                available_choices: vec![
                    ConflictChoice::KeepTarget,
                    ConflictChoice::RemoveWithBackup,
                ],
            }),
            None,
        ),
    };
    let file_changes: Vec<FileChange> = t_manifest
        .iter()
        .map(|t| FileChange {
            kind: "delete".into(),
            rel_path: t.rel_path.clone(),
            bytes: t.size,
            source_digest: None,
            target_digest: Some(t.sha256.clone()),
            diff_previewable: false,
        })
        .collect();
    Ok(PlanItem {
        item_id: uuid::Uuid::new_v4().to_string(),
        mapping_id: mapping.id.clone(),
        skill_id: skill.id.clone(),
        skill_name: skill.name.clone().unwrap_or_else(|| mapping.target_dir_name.clone()),
        rel_path: skill.rel_path.clone(),
        action,
        file_changes,
        excluded_by_ignore: Vec::new(),
        needs_backup,
        target_path: target_dir.to_string_lossy().to_string(),
        state_before: if managed_remove_blocked(&blocked_reason) {
            MatrixCellState::UnmanagedConflict
        } else {
            MatrixCellState::Synced
        },
        conflict,
        blocked_reason,
        selected,
        decision: None,
        source_digest: None,
        target_digest: t_digest,
        baseline_digest: baseline.map(|b| b.digest),
        restore: None,
    })
}

fn managed_remove_blocked(blocked: &Option<String>) -> bool {
    blocked.is_some()
}

// ---------------------------------------------------------------------------
// 指纹与失效判定（§7.2）
// ---------------------------------------------------------------------------

pub fn compute_fingerprint(
    config_version: i64,
    operation: PlanOperation,
    items: &[PlanItem],
) -> String {
    let mut h = Sha256::new();
    h.update(config_version.to_string().as_bytes());
    h.update(b"\n");
    h.update(serde_json::to_string(&operation).unwrap_or_default().as_bytes());
    let mut sorted: Vec<&PlanItem> = items.iter().collect();
    sorted.sort_by(|a, b| a.mapping_id.cmp(&b.mapping_id));
    for i in sorted {
        h.update(b"\nitem\n");
        h.update(i.mapping_id.as_bytes());
        h.update(serde_json::to_string(&i.action).unwrap_or_default().as_bytes());
        h.update(serde_json::to_string(&i.decision).unwrap_or_default().as_bytes());
        h.update(i.source_digest.as_deref().unwrap_or("<absent>").as_bytes());
        h.update(i.target_digest.as_deref().unwrap_or("<absent>").as_bytes());
        h.update(
            serde_json::to_string(&i.baseline_digest)
                .unwrap_or_default()
                .as_bytes(),
        );
        if let Some(r) = &i.restore {
            h.update(r.snapshot_id.as_bytes());
            h.update(r.to_absent.to_string().as_bytes());
        }
    }
    hex::encode(h.finalize())
}

/// 执行前复核（§7.2/§8.4.3）：配置版本、计划状态与逐项 S/T 摘要。
pub fn validate_for_execute(store: &Store, plan_id: &str, plan_version: i64) -> AppResult<PlanRow> {
    let plan = store.get_plan(plan_id)?;
    if plan.version != plan_version {
        return Err(AppError::new(
            ErrorCode::PlanStale,
            "计划版本不匹配：内容在预览后发生了变化，请刷新预览",
        ));
    }
    if plan.status != PlanStatus::Active {
        return Err(AppError::new(
            ErrorCode::PlanStale,
            format!("计划已失效（状态：{:?}），请重新生成预览", plan.status),
        ));
    }
    let library = store.get_library(&plan.library_id)?;
    if library.config_version != plan.config_version {
        store.set_plan_status(plan_id, PlanStatus::Stale)?;
        return Err(AppError::new(
            ErrorCode::PlanStale,
            "忽略规则等配置已变化，请刷新预览",
        ));
    }
    let source_root = PathBuf::from(library.source_root.clone().unwrap_or_default());
    let ignore = scanner::build_globset(&library.ignore_patterns)?;
    for item in &plan.items {
        if !item.selected || item.action == PlanAction::Skip || item.action == PlanAction::Blocked {
            continue;
        }
        let mapping = store.get_mapping(&item.mapping_id)?;
        let physical = store.get_physical_target(&mapping.physical_target_id)?;
        let target_dir = PathBuf::from(&physical.canonical_path).join(&mapping.target_dir_name);
        let (t_digest, _) = scanner::digest_directory(&target_dir)?
            .map(|(d, _)| (Some(d), ()))
            .unwrap_or((None, ()));
        // 恢复项的内容来源是静态快照（§7.4），源库不回退——只复核目标摘要
        let source_ok = if item.action == PlanAction::Restore || plan.operation == PlanOperation::Restore {
            true
        } else {
            let skill = store.get_skill(&item.skill_id)?;
            let (s_digest, _, _, _) = live_source(&skill, &source_root, &ignore)?;
            s_digest == item.source_digest
        };
        if !source_ok || t_digest != item.target_digest {
            store.set_plan_status(plan_id, PlanStatus::Stale)?;
            return Err(AppError::new(
                ErrorCode::PlanStale,
                format!("「{}」的内容在预览后发生了变化，请刷新预览", item.skill_name),
            )
            .with_context(serde_json::json!({
                "skillName": item.skill_name,
                "targetPath": item.target_path,
            })));
        }
    }
    Ok(plan)
}

// ---------------------------------------------------------------------------
// 冲突解决（§6.3/§8.2：明确选择只对当前内容与当前计划有效）
// ---------------------------------------------------------------------------

pub fn resolve_conflict(
    store: &Store,
    plan_id: &str,
    plan_version: i64,
    item_id: &str,
    choice: ConflictChoice,
) -> AppResult<SyncPlan> {
    let plan = store.get_plan(plan_id)?;
    if plan.version != plan_version {
        return Err(AppError::new(
            ErrorCode::PlanStale,
            "计划版本不匹配：他处已修改该计划，请刷新",
        ));
    }
    if plan.status != PlanStatus::Active {
        return Err(AppError::new(ErrorCode::PlanStale, "计划已失效，请重新生成预览"));
    }
    let mut items = plan.items.clone();
    {
        let item = items
            .iter_mut()
            .find(|i| i.item_id == item_id)
            .ok_or_else(|| AppError::not_found("计划项", item_id))?;
        apply_choice(store, &plan, item, choice)?;
    }
    let fingerprint = compute_fingerprint(plan.config_version, plan.operation, &items);
    store.update_plan_items(plan_id, &items, &fingerprint)?;
    let row = store.get_plan(plan_id)?;
    store.plan_view(&row)
}

/// 批量冲突解决（§6.3）：对指定冲突类型的所有未决项应用同一选择。
/// 只处理「该选择在其 availableChoices 中」的项；其余跳过并计数返回。
/// 整批一次版本递增（不是逐项递增），指纹一次重算。
pub fn resolve_conflicts_bulk(
    store: &Store,
    plan_id: &str,
    plan_version: i64,
    kinds: &[String],
    choice: ConflictChoice,
) -> AppResult<(SyncPlan, BulkResolveOutcome)> {
    let plan = store.get_plan(plan_id)?;
    if plan.version != plan_version {
        return Err(AppError::new(
            ErrorCode::PlanStale,
            "计划版本不匹配：他处已修改该计划，请刷新",
        ));
    }
    if plan.status != PlanStatus::Active {
        return Err(AppError::new(ErrorCode::PlanStale, "计划已失效，请重新生成预览"));
    }
    let mut items = plan.items.clone();
    let mut applied: Vec<String> = Vec::new();
    let mut skipped: Vec<String> = Vec::new();
    for item in items.iter_mut() {
        let Some(conflict) = item.conflict.clone() else {
            continue;
        };
        if !kinds.contains(&conflict.kind) {
            continue;
        }
        if !conflict.available_choices.contains(&choice) {
            skipped.push(item.item_id.clone());
            continue;
        }
        apply_choice(store, &plan, item, choice)?;
        applied.push(item.item_id.clone());
    }
    if !applied.is_empty() {
        let fingerprint = compute_fingerprint(plan.config_version, plan.operation, &items);
        store.update_plan_items(plan_id, &items, &fingerprint)?;
    }
    let row = store.get_plan(plan_id)?;
    let view = store.plan_view(&row)?;
    Ok((view, BulkResolveOutcome { applied, skipped }))
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkResolveOutcome {
    pub applied: Vec<String>,
    pub skipped: Vec<String>,
}

/// 把单个冲突选择落到计划项上（单项与批量共用的核心转移逻辑）。
fn apply_choice(
    store: &Store,
    plan: &PlanRow,
    item: &mut PlanItem,
    choice: ConflictChoice,
) -> AppResult<()> {
    let conflict = item.conflict.clone().ok_or_else(|| {
        AppError::new(ErrorCode::ValidationFailed, "该计划项没有待处理的冲突")
    })?;
    if !conflict.available_choices.contains(&choice) {
        return Err(AppError::new(
            ErrorCode::ValidationFailed,
            format!("该冲突不支持选择 {choice:?}"),
        ));
    }
    // 归属转移：先落库再按当前内容重新判定
    if choice == ConflictChoice::TransferOwnership {
        let mapping = store.get_mapping(&item.mapping_id)?;
        store.transfer_mapping_ownership(&mapping.id, &plan.library_id)?;
    }
    match choice {
        ConflictChoice::AdoptExisting => {
            item.action = PlanAction::Adopt;
            item.selected = true;
            item.needs_backup = false;
        }
        ConflictChoice::TakeOver | ConflictChoice::TransferOwnership => {
            item.action = PlanAction::TakeOver;
            item.selected = true;
            item.needs_backup = true; // 接管不同内容的目录也必须先备份（§7.4）
        }
        ConflictChoice::OverwriteWithSource => {
            item.action = if plan.operation == PlanOperation::Restore {
                PlanAction::Restore
            } else {
                PlanAction::Overwrite
            };
            item.selected = true;
            item.needs_backup = true;
        }
        ConflictChoice::RemoveWithBackup => {
            item.action = PlanAction::Remove;
            item.selected = true;
            item.needs_backup = true;
        }
        ConflictChoice::Reinstall => {
            item.action = PlanAction::Reinstall;
            item.selected = true;
            item.needs_backup = false;
        }
        ConflictChoice::KeepTarget | ConflictChoice::KeepDeleted => {
            item.action = PlanAction::Skip;
            item.selected = false;
        }
        ConflictChoice::PauseMapping => {
            store.set_mapping_paused(&item.mapping_id, Some("user"))?;
            item.action = PlanAction::Skip;
            item.selected = false;
        }
    }
    item.decision = Some(choice);
    item.conflict = None;
    // 重新计算文件变化（接管/覆盖需要完整差异）
    if matches!(
        item.action,
        PlanAction::Overwrite | PlanAction::TakeOver | PlanAction::Reinstall
    ) {
        let library = store.get_library(&plan.library_id)?;
        let source_root = PathBuf::from(library.source_root.clone().unwrap_or_default());
        let ignore = scanner::build_globset(&library.ignore_patterns)?;
        let skill = store.get_skill(&item.skill_id)?;
        let (s_digest, s_manifest, _, _) = live_source(&skill, &source_root, &ignore)?;
        let (_, t_manifest) = scanner::digest_directory_cached(Path::new(&item.target_path))?
            .map(|(d, m)| (Some(d), m))
            .unwrap_or((None, Vec::new()));
        item.file_changes = diff_manifests(
            &s_manifest,
            &t_manifest,
            Some(&source_root.join(&skill.rel_path)),
        );
        item.source_digest = s_digest;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decision_table_rows() {
        // 无托管
        let x = decide(false, Content::Present("a"), Content::Absent, Content::Absent, ValidationStatus::Valid, false, false);
        assert_eq!((x.state, x.action, x.selected), (MatrixCellState::ToAdd, PlanAction::Create, true));

        let x = decide(false, Content::Present("a"), Content::Present("a"), Content::Absent, ValidationStatus::Valid, false, false);
        assert_eq!(x.state, MatrixCellState::SameContentUnmanaged);
        assert_eq!(x.action, PlanAction::Skip);

        let x = decide(false, Content::Present("a"), Content::Present("b"), Content::Absent, ValidationStatus::Valid, false, false);
        assert_eq!(x.state, MatrixCellState::UnmanagedConflict);
        assert!(!x.selected, "非托管同名目录默认保留（不静默覆盖）");

        // 有基线
        let x = decide(true, Content::Present("a"), Content::Present("a"), Content::Present("a"), ValidationStatus::Valid, false, false);
        assert_eq!((x.state, x.action), (MatrixCellState::Synced, PlanAction::Skip));

        let x = decide(true, Content::Present("b"), Content::Present("a"), Content::Present("a"), ValidationStatus::Valid, false, false);
        assert_eq!((x.state, x.action, x.selected, x.needs_backup), (MatrixCellState::SourceUpdated, PlanAction::Update, true, true));

        let x = decide(true, Content::Present("a"), Content::Present("b"), Content::Present("a"), ValidationStatus::Valid, false, false);
        assert_eq!(x.state, MatrixCellState::TargetModified);
        assert_eq!(x.action, PlanAction::Skip);

        let x = decide(true, Content::Present("c"), Content::Present("b"), Content::Present("a"), ValidationStatus::Valid, false, false);
        assert_eq!(x.state, MatrixCellState::BothModified);

        let x = decide(true, Content::Present("b"), Content::Present("b"), Content::Present("a"), ValidationStatus::Valid, false, false);
        assert_eq!((x.state, x.action), (MatrixCellState::AlignedExternally, PlanAction::Adopt));

        let x = decide(true, Content::Present("a"), Content::Absent, Content::Present("a"), ValidationStatus::Valid, false, false);
        assert_eq!(x.state, MatrixCellState::TargetDeleted);
        assert!(!x.selected, "目标已删除默认保留删除状态");

        // 源已移除：默认保留，不进可执行项
        let x = decide(true, Content::Absent, Content::Present("a"), Content::Present("a"), ValidationStatus::Valid, false, false);
        assert_eq!((x.state, x.action), (MatrixCellState::SourceRemoved, PlanAction::Skip));

        // 归属冲突最先判定
        let x = decide(false, Content::Present("a"), Content::Present("b"), Content::Absent, ValidationStatus::Valid, false, true);
        assert_eq!(x.state, MatrixCellState::OwnershipConflict);

        // 校验失败为阻塞项
        let x = decide(false, Content::Present("a"), Content::Absent, Content::Absent, ValidationStatus::Invalid, false, false);
        assert_eq!(x.action, PlanAction::Blocked);
        let x = decide(false, Content::Present("a"), Content::Absent, Content::Absent, ValidationStatus::Valid, true, false);
        assert_eq!(x.state, MatrixCellState::Paused);
    }
}
