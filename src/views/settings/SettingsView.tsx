import React, { useState, useEffect } from 'react';
import { useApp } from '../../context/AppContext';
import { Icon } from '../../components/common/Icon';
import { invokeCommand } from '../../services/api';
import type { BackupStats } from '../../lib/backend-contract';

export const SettingsView: React.FC = () => {
  const { activeLibrary, reloadCurrentLibrary, showToast } = useApp();

  // 01 排除规则草稿
  const [rules, setRules] = useState<string[]>([]);
  const [newRule, setNewRule] = useState('');
  const [rulesDirty, setRulesDirty] = useState(false);
  const [savingRules, setSavingRules] = useState(false);

  // 02 备份设置草稿
  const [backupStats, setBackupStats] = useState<BackupStats | null>(null);
  const [retentionDays, setRetentionDays] = useState(30);
  const [capGB, setCapGB] = useState(2);
  const [backupDirty, setBackupDirty] = useState(false);
  const [savingBackup, setSavingBackup] = useState(false);

  // 同步当前库的排除规则
  useEffect(() => {
    if (activeLibrary) {
      setRules(activeLibrary.ignorePatterns || []);
      setRulesDirty(false);
    }
  }, [activeLibrary]);

  // 加载备份配置
  useEffect(() => {
    invokeCommand('get_backup_stats')
      .then((stats) => {
        setBackupStats(stats);
        setRetentionDays(stats.retentionDays);
        setCapGB(stats.softCapBytes / 1073741824);
        setBackupDirty(false);
      })
      .catch((e) => {
        console.error('获取备份配置失败:', e);
      });
  }, []);

  const handleAddRule = (e?: React.FormEvent) => {
    if (e) e.preventDefault();
    const val = newRule.trim();
    if (!val) return;
    if (rules.includes(val)) {
      showToast('该规则已存在', 'warning');
      return;
    }
    setRules([...rules, val]);
    setNewRule('');
    setRulesDirty(true);
  };

  const handleRemoveRule = (index: number) => {
    const updated = rules.filter((_, i) => i !== index);
    setRules(updated);
    setRulesDirty(true);
  };

  const handleSaveRules = async () => {
    if (!activeLibrary) return;
    setSavingRules(true);
    try {
      await invokeCommand('update_library_settings', {
        libraryId: activeLibrary.summary.libraryId,
        ignorePatterns: rules,
      });
      setRulesDirty(false);
      await reloadCurrentLibrary();
      showToast('已保存排除规则，已自动刷新技能库状态', 'success');
    } catch (e: any) {
      showToast(`保存规则失败: ${e?.message || e}`, 'error');
    } finally {
      setSavingRules(false);
    }
  };

  const handleSaveBackup = async () => {
    setSavingBackup(true);
    try {
      const updated = await invokeCommand('update_backup_settings', {
        retentionDays: Number(retentionDays),
        softCapBytes: Math.round(Number(capGB) * 1073741824),
      });
      setBackupStats(updated);
      setBackupDirty(false);
      showToast('已保存备份与恢复策略', 'success');
    } catch (e: any) {
      showToast(`保存备份策略失败: ${e?.message || e}`, 'error');
    } finally {
      setSavingBackup(false);
    }
  };

  const formatSize = (bytes: number) => {
    if (bytes >= 1073741824) return `${(bytes / 1073741824).toFixed(1)} GB`;
    if (bytes >= 1048576) return `${(bytes / 1048576).toFixed(0)} MB`;
    if (bytes >= 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${bytes} B`;
  };

  return (
    <div className="page">
      {/* 顶部标题 */}
      <div className="page-heading">
        <div>
          <div className="eyebrow">PREFERENCES</div>
          <h1>设置</h1>
          <p>简单几项，适合你的使用习惯。</p>
        </div>
      </div>

      <div className="settings-content">
        {/* 01 文件排除规则 */}
        <section className="settings-section">
          <div className="settings-section-heading">
            <span className="section-number">01</span>
            <div>
              <h2>文件排除规则</h2>
              <p>
                {activeLibrary
                  ? `应用于 ${activeLibrary.summary.displayName}，匹配的文件不会被分发到各目标。`
                  : '请先添加或选择技能库，再设置文件排除规则。'}
              </p>
            </div>
          </div>

          <div className="setting-body">
            <div className="rules">
              {rules.map((r, i) => (
                <span key={i} className="rule">
                  <code>{r}</code>
                  <button
                    type="button"
                    onClick={() => handleRemoveRule(i)}
                    aria-label={`移除规则 ${r}`}
                    title="移除"
                  >
                    <Icon name="close" size={11} />
                  </button>
                </span>
              ))}
            </div>

            <form onSubmit={handleAddRule} className="rule-input">
              <input
                id="rule-input"
                type="text"
                placeholder="例如 **/*.log、node_modules/**"
                value={newRule}
                disabled={!activeLibrary}
                onChange={(e) => setNewRule(e.target.value)}
                aria-label="新的忽略规则"
              />
              <button
                type="submit"
                className="button secondary small"
                disabled={!activeLibrary || !newRule.trim()}
              >
                <Icon name="plus" size={12} />
                <span>添加</span>
              </button>
            </form>

            <div className="setting-section-footer">
              <span className="save-hint">
                {rulesDirty ? '有未保存的排除规则更改' : ''}
              </span>
              <button
                type="button"
                className="button secondary small"
                disabled={!rulesDirty || savingRules}
                onClick={handleSaveRules}
              >
                {savingRules ? '保存中...' : '保存规则'}
              </button>
            </div>
          </div>
        </section>

        {/* 02 备份与恢复 */}
        <section className="settings-section">
          <div className="settings-section-heading">
            <span className="section-number">02</span>
            <div>
              <h2>备份与恢复</h2>
              <p>更新目标文件前，自动保存修改前的完整版本。</p>
            </div>
          </div>

          <div className="setting-body">
            <div className="setting-controls">
              <div className="field">
                <label htmlFor="retention-days">备份保留时间</label>
                <div className="unit-input">
                  <input
                    id="retention-days"
                    type="number"
                    min="1"
                    step="1"
                    value={retentionDays}
                    onChange={(e) => {
                      setRetentionDays(Number(e.target.value));
                      setBackupDirty(true);
                    }}
                  />
                  <span>天</span>
                </div>
                <p>按保留策略清理可释放的历史备份。</p>
              </div>

              <div className="field">
                <label htmlFor="backup-cap">备份容量软上限</label>
                <div className="unit-input">
                  <input
                    id="backup-cap"
                    type="number"
                    min="0.1"
                    step="0.1"
                    value={capGB}
                    onChange={(e) => {
                      setCapGB(Number(e.target.value));
                      setBackupDirty(true);
                    }}
                  />
                  <span>GB</span>
                </div>
                <p>
                  当前已用 {backupStats ? formatSize(backupStats.totalBytes) : '0 MB'}
                  ，受保护备份将继续保留。
                </p>
              </div>
            </div>

            <div className="setting-section-footer">
              <span className="save-hint">
                {backupDirty ? '有未保存的备份策略更改' : ''}
              </span>
              <button
                type="button"
                className="button secondary small"
                disabled={!backupDirty || savingBackup}
                onClick={handleSaveBackup}
              >
                {savingBackup ? '保存中...' : '保存策略'}
              </button>
            </div>
          </div>
        </section>

        {/* 03 键盘快捷键 */}
        <section className="settings-section">
          <div className="settings-section-heading">
            <span className="section-number">03</span>
            <div>
              <h2>键盘快捷键</h2>
              <p>把常用操作，留在手边。</p>
            </div>
          </div>

          <div className="shortcut-grid select-none">
            <div className="shortcut">
              <span>添加技能库</span>
              <kbd>Ctrl O</kbd>
            </div>
            <div className="shortcut">
              <span>搜索技能</span>
              <kbd>Ctrl F</kbd>
            </div>
            <div className="shortcut">
              <span>刷新当前页面</span>
              <kbd>F5</kbd>
            </div>
            <div className="shortcut">
              <span>打开同步预览</span>
              <kbd>Ctrl Enter</kbd>
            </div>
            <div className="shortcut">
              <span>关闭详情或弹窗</span>
              <kbd>Esc</kbd>
            </div>
          </div>
        </section>
      </div>
    </div>
  );
};
