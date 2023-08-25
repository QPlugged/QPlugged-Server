import debug from "debug";

const logger = debug("debugger");

export function initDebugger() {
    (process as any)._debugProcess(process.pid);
    logger("已在端口 %d 上启动 Node.js 调试器", process.debugPort);
}
