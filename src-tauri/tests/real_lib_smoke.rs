//! 真实样例库的只读冒烟测试（默认不运行）。
//!
//! 运行方式（只扫描，不写入源目录）：
//!   SKILLDOCK_SMOKE_SRC=D:\code\SkillDock\skills cargo test --test real_lib_smoke -- --ignored
//!
//! 所有应用数据仍写入独立临时目录；源目录全程只读。

use skilldock_lib::scanner::{self, ScanOptions, ValidationStatus};
use skilldock_lib::storage::Store;
use skilldock_lib::windows_paths;
use std::path::PathBuf;

#[test]
#[ignore = "需要 SKILLDOCK_SMOKE_SRC 指向真实样例库"]
fn scans_real_skill_library_readonly() {
    let Some(src) = std::env::var_os("SKILLDOCK_SMOKE_SRC").map(PathBuf::from) else {
        eprintln!("未设置 SKILLDOCK_SMOKE_SRC，跳过");
        return;
    };
    assert!(src.is_dir(), "SMOKE_SRC 不存在");
    let before = std::fs::read_dir(&src).unwrap().count();

    let started = std::time::Instant::now();
    let scanned = scanner::scan_source_root(
        &src,
        &ScanOptions { ignore: None, skill_filter: None, on_dir: None },
    )
    .unwrap();
    let elapsed = started.elapsed();

    let valid = scanned.iter().filter(|s| s.status == ValidationStatus::Valid).count();
    let invalid = scanned.iter().filter(|s| s.status == ValidationStatus::Invalid).count();
    let unsupported = scanned
        .iter()
        .filter(|s| s.status == ValidationStatus::Unsupported)
        .count();
    let total_files: u32 = scanned.iter().map(|s| s.file_count).sum();
    let total_bytes: u64 = scanned.iter().map(|s| s.total_bytes).sum();
    eprintln!(
        "扫描 {} 个技能（valid {valid} / invalid {invalid} / unsupported {unsupported}），\
         {total_files} 文件 / {total_bytes} 字节，耗时 {elapsed:?}",
        scanned.len()
    );
    assert!(scanned.len() > 100, "样例库应有上百个技能，实得 {}", scanned.len());
    assert!(valid > 100, "绝大多数技能应校验通过，实得 {valid}");

    // 无效项必须带定位信息
    for s in scanned.iter().filter(|s| s.status != ValidationStatus::Valid) {
        eprintln!("  [{}] {}: {:?}", s.dir_name, s.rel_path, s.status);
        for i in &s.issues {
            eprintln!("      - {} (行 {:?}): {}", i.code, i.line, i.message);
        }
    }

    // 落库（临时数据目录）并验证幂等重扫
    let tmp = tempfile::tempdir().unwrap();
    let store = Store::open(&tmp.path().join("appdata")).unwrap();
    let lib_id = uuid::Uuid::new_v4().to_string();
    let canonical = windows_paths::canonicalize(&src).unwrap();
    store
        .insert_library(
            &lib_id, "冒烟库", &canonical.to_string_lossy(), Some(&canonical.to_string_lossy()),
            Some(&windows_paths::fold_case(&canonical)), "collection", None,
        )
        .unwrap();
    store.apply_scan(&lib_id, &scanned).unwrap();
    let again = scanner::scan_source_root(
        &src,
        &ScanOptions { ignore: None, skill_filter: None, on_dir: None },
    )
    .unwrap();
    assert_eq!(scanned.len(), again.len());
    for (a, b) in scanned.iter().zip(again.iter()) {
        assert_eq!(a.digest, b.digest, "重复扫描摘要应稳定：{}", a.rel_path);
    }

    // 源目录未被写入（只读保证）
    let after = std::fs::read_dir(&src).unwrap().count();
    assert_eq!(before, after, "扫描不得在源目录创建任何内容");
}
