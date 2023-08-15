import { findFreePort } from "./freePort";

// @ts-expect-error
export const resourceDir: string = global.__QP_DIR;
// @ts-expect-error
export const isProduction: boolean = IS_PROD;

export const isInspectorMode =
    (process.env.QP_INSPECTOR !== undefined &&
        !!parseInt(process.env.QP_INSPECTOR)) ||
    false;
export const listenPort =
    (process.env.QP_PORT !== undefined && parseInt(process.env.QP_PORT)) ||
    (isProduction ? findFreePort() : 15321);
