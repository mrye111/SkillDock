# 提示词模式（Prompt Patterns）

> 使用条件：仅当请求需要比 SKILL.md 更多的塑造时使用（多参考角色、有序动作节拍、声音时间或修复重写）。
> 少量示例文本是英文的（为简洁）；将最终标签和内容翻译为用户语言。

## 结构化提示词规则

- 始终将最终提示词写成结构化提示词主体
- 简单请求可以使用更少行，但仍保持可见章节标签
- 运行时可用性、参数名和执行状态不要放入创意提示词

## 英文 few-shot 模式

结构化提示词骨架：

```
Style/outcome: [tone, genre, and desired video result]. Subject/scene: [main subject, setting, and atmosphere]. Action beats: [one clear action path, natural motion, and final moment]. Audio: [dialogue, ambience, music, or silence]. Preserve: [identity, props, layout, lighting logic, and key continuity locks]. Avoid: [main failure risk].
```

参考角色：

```
Reference roles: - {{Image 1}}: [identity, product, layout, or first frame]. - {{Image 2}}: [environment, style, prop, or end-state cue]. - Reference video: [pacing, camera rhythm, motion mechanism, or shot structure]. Keep each role narrow. Avoid blending unrelated details across references.
```

时间线：

```
0-2s: establish [subject and setting] with [opening action or mood cue]. 2-4s: [main change, reaction, reveal, or interaction] happens clearly. 4-5s: settle on [final state or memorable frame]. Sound priority: [dialogue, ambience, music, or silence].
```

## 修复模式

| 问题 | 修复方法 |
|------|---------|
| Over-specified result（过度指定） | 保持相同章节标签，但移除次要动作 |
| Under-specified result（指定不足） | 添加主体、场景、预期结果、一个动作路径和终态 |
| Reference confusion（参考混淆） | 使用 Reference roles 并避免共享控制 |
| Dialogue overload（对白超载） | 保留一行、一个停顿和一个可见反应 |
| Chaotic pacing（节奏混乱） | 添加 Timeline 章节并定义一个镜头行为 |
