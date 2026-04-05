import { execFileSync } from "child_process";
import {
  detectFrameworkFromImage,
  isDevProcess,
  normalizePortEntry,
  normalizeProcessEntry,
} from "./scanner-shared.js";

const POWERSHELL = "powershell.exe";

function runPowerShellJson(script) {
  try {
    const raw = execFileSync(
      POWERSHELL,
      ["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", script],
      {
        encoding: "utf8",
        timeout: 15000,
        windowsHide: true,
      },
    ).trim();

    if (!raw) return null;
    return JSON.parse(raw);
  } catch {
    return null;
  }
}

function getDockerInfo() {
  const map = new Map();
  try {
    const raw = execFileSync(
      "docker",
      ["ps", "--format", "{{.Ports}}\t{{.Names}}\t{{.Image}}"],
      {
        encoding: "utf8",
        timeout: 5000,
        windowsHide: true,
      },
    ).trim();

    for (const line of raw.split("\n")) {
      if (!line.trim()) continue;
      const [portsStr, name, image] = line.split("\t");
      if (!portsStr || !name) continue;

      const matches = portsStr.matchAll(
        /(?:\d+\.\d+\.\d+\.\d+|\[::\]|::):(\d+)->/g,
      );
      const seen = new Set();
      for (const match of matches) {
        const port = parseInt(match[1], 10);
        if (!seen.has(port)) {
          seen.add(port);
          map.set(port, { name, image });
        }
      }
    }
  } catch {}

  return map;
}

function getSnapshot() {
  const data = runPowerShellJson(`
$ErrorActionPreference = 'SilentlyContinue'
$connections = @(Get-NetTCPConnection -State Listen | Select-Object LocalPort, OwningProcess)
$processes = @(Get-CimInstance Win32_Process | Select-Object ProcessId, ParentProcessId, Name, CommandLine, ExecutablePath, CreationDate, WorkingSetSize)
$perf = @(Get-CimInstance Win32_PerfFormattedData_PerfProc_Process | Select-Object IDProcess, PercentProcessorTime)

$payload = [PSCustomObject]@{
  Connections = $connections
  Processes = $processes
  Perf = $perf
}

$payload | ConvertTo-Json -Depth 6 -Compress
`);

  if (!data) {
    return {
      portEntries: [],
      processesByPid: new Map(),
      cpuByPid: new Map(),
    };
  }

  const connections = Array.isArray(data.Connections)
    ? data.Connections
    : data.Connections
      ? [data.Connections]
      : [];
  const processList = Array.isArray(data.Processes)
    ? data.Processes
    : data.Processes
      ? [data.Processes]
      : [];
  const perfList = Array.isArray(data.Perf)
    ? data.Perf
    : data.Perf
      ? [data.Perf]
      : [];

  const processesByPid = new Map();
  for (const proc of processList) {
    processesByPid.set(Number(proc.ProcessId), {
      pid: Number(proc.ProcessId),
      ppid: Number(proc.ParentProcessId),
      processName: proc.Name || "unknown",
      command: proc.CommandLine || "",
      executablePath: proc.ExecutablePath || null,
      startTime: proc.CreationDate ? new Date(proc.CreationDate) : null,
      rssKB: proc.WorkingSetSize ? Number(proc.WorkingSetSize) / 1024 : null,
    });
  }

  const cpuByPid = new Map();
  for (const perf of perfList) {
    cpuByPid.set(Number(perf.IDProcess), Number(perf.PercentProcessorTime) || 0);
  }

  const seenPorts = new Set();
  const portEntries = [];
  for (const conn of connections) {
    const port = Number(conn.LocalPort);
    const pid = Number(conn.OwningProcess);
    if (!port || seenPorts.has(port)) continue;
    seenPorts.add(port);
    portEntries.push({ port, pid });
  }

  return { portEntries, processesByPid, cpuByPid };
}

function buildProcessTree(pid, processesByPid) {
  const tree = [];
  let currentPid = pid;
  let depth = 0;

  while (currentPid > 0 && depth < 8) {
    const proc = processesByPid.get(currentPid);
    if (!proc) break;
    tree.push({ pid: proc.pid, ppid: proc.ppid, name: proc.processName });
    currentPid = proc.ppid;
    depth++;
  }

  return tree;
}

export function getListeningPorts(detailed = false) {
  const { portEntries, processesByPid } = getSnapshot();
  const dockerInfo = getDockerInfo();

  return portEntries
    .map(({ port, pid }) => {
      const proc = processesByPid.get(pid);
      if (!proc) return null;

      const docker = dockerInfo.get(port);
      const parentKnown = proc.ppid > 0 && processesByPid.has(proc.ppid);
      const base = {
        port,
        pid,
        processName: docker ? "docker" : proc.processName,
        rawName: proc.processName,
        command: proc.command,
        cwd: null,
        executablePath: proc.executablePath,
        framework: docker
          ? detectFrameworkFromImage(docker.image)
          : null,
        projectName: docker ? docker.name : null,
        status: !parentKnown && isDevProcess(proc.processName, proc.command)
          ? "orphaned"
          : "healthy",
        rssKB: proc.rssKB,
        startTime: proc.startTime,
        processTree: detailed ? buildProcessTree(pid, processesByPid) : [],
      };

      return normalizePortEntry(base, detailed);
    })
    .filter(Boolean)
    .sort((a, b) => a.port - b.port);
}

export function getPortDetails(targetPort) {
  const ports = getListeningPorts(true);
  return ports.find((port) => port.port === targetPort) || null;
}

export function getAllProcesses() {
  const { processesByPid, cpuByPid } = getSnapshot();

  return [...processesByPid.values()]
    .filter((proc) => proc.pid > 1 && proc.pid !== process.pid)
    .map((proc) =>
      normalizeProcessEntry({
        ...proc,
        cwd: null,
        cpu: cpuByPid.get(proc.pid) || 0,
      }),
    );
}

export function findOrphanedProcesses() {
  return getListeningPorts().filter((port) => port.status === "orphaned");
}

export function killProcess(pid, signal = "SIGTERM") {
  try {
    process.kill(pid, signal);
    return true;
  } catch {
    return false;
  }
}

export function watchPorts(callback, intervalMs = 2000) {
  let previousPorts = new Set();

  const check = () => {
    const current = getListeningPorts();
    const currentSet = new Set(current.map((port) => port.port));

    for (const port of current) {
      if (!previousPorts.has(port.port)) {
        callback("new", port);
      }
    }

    for (const port of previousPorts) {
      if (!currentSet.has(port)) {
        callback("removed", { port });
      }
    }

    previousPorts = currentSet;
  };

  check();
  return setInterval(check, intervalMs);
}
