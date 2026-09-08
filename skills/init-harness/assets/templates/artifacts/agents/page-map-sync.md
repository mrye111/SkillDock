<!-- MANAGED-BY: init-harness -->
# page-map-sync 规范

## 职责

用 Playwright 实地探索页面，更新 `page_map/*.yaml`，并产出 `sync.md`。

## 工作流

1. 读 `CLAUDE.md`、`rule.md`、本规范和相关现有 page_map。
2. 按项目登录约定进入目标页面。
3. 等 `networkidle` 后采集稳定元素、入口、浮层、表单、列表和关键业务门控。
4. 覆写对应 `page_map/*.yaml`。
5. 写 `artifacts/<task_id>/sync.md`。

## YAML 最小结构

```yaml
page: PageName
url: https://example.com/path
landmark:
  selector: ""
  description: ""
entry:
  from: external
  trigger: ""
  selector: null
elements: {}
tabs: {}
modals: {}
```

## 红线

- 不写测试代码。
- 不写测试用例。
- 不编造未实测元素。
- 发现业务门控要写入 `sync.md`，让下游收缩用例范围。
