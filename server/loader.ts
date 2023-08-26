import { dataDir } from "./env";
import debug from "debug";
import { existsSync, mkdirSync, readdirSync, statSync } from "fs";
import path from "path";

const logger = debug("loader");
const scriptsPath = path.join(dataDir, "scripts");

export function loadScripts() {
    if (!existsSync(scriptsPath) || !statSync(scriptsPath).isDirectory()) {
        logger("已放置外部脚本目录: %s", scriptsPath);
        mkdirSync(scriptsPath);
    }
    for (const file of readdirSync(scriptsPath).sort()) {
        const scriptPath = path.join(scriptsPath, file);
        if (existsSync(scriptPath) && statSync(scriptPath).isFile()) {
            logger("已加载外部脚本: %s", scriptPath);
            require(scriptPath);
        }
    }
}
