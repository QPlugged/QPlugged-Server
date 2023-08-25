import { initDebugger } from "./debugger";
import { listenPort } from "./env";
import { loadScripts } from "./loader";
import { patchModuleLoader } from "./modules";
import { registerPatch } from "./server";
import debug from "debug";
import { app } from "electron";

const logger = debug("main");

app.disableHardwareAcceleration();

logger("远程连接端口: %d", listenPort);
initDebugger();
patchModuleLoader();
registerPatch();
loadScripts();

// if (process.platform !== "linux") {
//     console.log = () => undefined;
//     console.info = () => undefined;
//     console.warn = () => undefined;
//     console.error = () => undefined;
// }
