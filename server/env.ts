import { findFreePort } from "./freePort";

// @ts-expect-error
export const resourceDir: string = global.__QP_RES_DIR;
// @ts-expect-error
export const dataDir: string = global.__QP_DATA_DIR;
// @ts-expect-error
export const isProduction: boolean = IS_PROD;

export const isInspectorMode =
    process.env.QP_SERVER_INSPECTOR !== undefined
        ? !!parseInt(process.env.QP_SERVER_INSPECTOR)
        : false;
export const listenPort =
    process.env.QP_SERVER_PORT !== undefined
        ? parseInt(process.env.QP_SERVER_PORT)
        : isProduction
        ? findFreePort()
        : 15321;
