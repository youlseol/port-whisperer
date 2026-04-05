import * as unixScanner from "./scanner-unix.js";
import * as windowsScanner from "./scanner-windows.js";
import { isDevProcess } from "./scanner-shared.js";

const scanner = process.platform === "win32" ? windowsScanner : unixScanner;

export const getListeningPorts = scanner.getListeningPorts;
export const getPortDetails = scanner.getPortDetails;
export const findOrphanedProcesses = scanner.findOrphanedProcesses;
export const killProcess = scanner.killProcess;
export const watchPorts = scanner.watchPorts;
export const getAllProcesses = scanner.getAllProcesses;
export { isDevProcess };
