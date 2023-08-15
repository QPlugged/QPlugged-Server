import { initDebugger } from "./debugger";
import { listenPort } from "./env";
import { patchModuleLoader } from "./modules";
import { registerPatch } from "./server";
import { app } from "electron";

app.disableHardwareAcceleration();

console.log(`[QPLUGGED_INIT_PORT]${listenPort}[/]`);
initDebugger();
patchModuleLoader();
registerPatch();

console.log = () => undefined;
console.info = () => undefined;
console.warn = () => undefined;
console.error = () => undefined;
