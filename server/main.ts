import { app } from "electron";
import { initDebugger } from "./debugger";
import { listenPort } from "./env";
import { patchModuleLoader } from "./modules";
import { registerPatch } from "./server";

app.disableHardwareAcceleration();

console.log(`QPlugged 远程连接端口: ${listenPort}`);
initDebugger();
patchModuleLoader();
registerPatch();

// if (process.platform !== "linux") {
//     console.log = () => undefined;
//     console.info = () => undefined;
//     console.warn = () => undefined;
//     console.error = () => undefined;
// }
