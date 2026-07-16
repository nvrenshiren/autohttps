// @ts-nocheck
// opcflow 生成的 OpenCode 插件:桥接壳,由 OpenCode(Bun)运行时直接执行,不参与宿主项目类型检查
// 把工具调用前后事件转发给 hook 脚本(观测写门禁 + 刷新 hash)
import { spawn } from "node:child_process"

function runHook(cmd: string, payload: unknown): Promise<void> {
  return new Promise(resolve => {
    const parts = cmd.split(" ")
    const p = spawn(parts[0], parts.slice(1), { stdio: ["pipe", "ignore", "ignore"] })
    p.on("error", () => resolve())
    p.on("close", () => resolve())
    p.stdin.write(JSON.stringify(payload))
    p.stdin.end()
  })
}

export const opcflow = async () => ({
  "tool.execute.before": async (_input: unknown, output: unknown) => {
    await runHook("npx -y @dawipong/opcflow hook pre --platform=opencode", output)
  },
  "tool.execute.after": async (_input: unknown, output: unknown) => {
    await runHook("npx -y @dawipong/opcflow hook post --platform=opencode", output)
  }
})
