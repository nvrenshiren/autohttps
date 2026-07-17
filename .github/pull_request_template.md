## 变更说明

<!-- 简述本 PR 做了什么、为什么。 -->

## 关联 issue

<!-- 如有:Closes #123 -->

## 测试 checklist

- [ ] `cargo check --workspace` 通过
- [ ] `cargo test` 通过
- [ ] `cd frontend && npm run build` 通过(`tsc --noEmit` + vite)
- [ ] 本地实测了受影响的功能(简述如何验证 / 贴关键输出)
- [ ] 涉及行为变更已同步更新文档 / README
- [ ] 未引入需求未明示的功能;敏感数据(私钥 / 账户密钥)未明文入库或入日志,DTO 未暴露 `*_ref`
