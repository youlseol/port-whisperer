import { execSync } from "child_process";
import { basename } from "path";
import {
  detectFrameworkFromCommand,
  detectFrameworkFromImage,
  findProjectRoot,
  formatMemory,
  formatUptime,
  getGitBranch,
  isDevProcess,
  normalizePortEntry,
  normalizeProcessEntry,
} from "./scanner-shared.js";

function batchPsInfo(pids) {
  const map = new Map();
  if (pids.length === 0) return map;

  try {
    const pidList = pids.join(",");
    const raw = execSync(
      `ps -p ${pidList} -o pid=,ppid=,stat=,rss=,lstart=,command= 2>/dev/null`,
      {
        encoding: "utf8",
        timeout: 5000,
      },
    ).trim();

    for (const line of raw.split("\n")) {
      if (!line.trim()) continue;
      const m = line
        .trim()
        .match(
          /^(\d+)\s+(\d+)\s+(\S+)\s+(\d+)\s+\w+\s+(\w+\s+\d+\s+[\d:]+\s+\d+)\s+(.*)$/,
        );
      if (!m) continue;
      map.set(parseInt(m[1], 10), {
        ppid: parseInt(m[2], 10),
        stat: m[3],
        rss: parseInt(m[4], 10),
        lstart: m[5],
        command: m[6],
      });
    }
  } catch {}
  return map;
}

function batchCwd(pids) {
  const map = new Map();
  if (pids.length === 0) return map;

  try {
    const pidList = pids.join(",");
    const raw = execSync(`lsof -a -d cwd -p ${pidList} 2>/dev/null`, {
      encoding: "utf8",
      timeout: 10000,
    }).trim();

    const lines = raw.split("\n").slice(1);
    for (const line of lines) {
      const parts = line.split(/\s+/);
      if (parts.length < 9) continue;
      const pid = parseInt(parts[1], 10);
      const path = parts.slice(8).join(" ");
      if (path && path.startsWith("/")) {
        map.set(pid, path);
      }
    }
  } catch {}
  return map;
}

function batchDockerInfo() {
  const map = new Map();
  try {
    const raw = execSync(
      'docker ps --format "{{.Ports}}\\t{{.Names}}\\t{{.Image}}" 2>/dev/null',
      {
        encoding: "utf8",
        timeout: 5000,
      },
    ).trim();

    for (const line of raw.split("\n")) {
      if (!line.trim()) continue;
      const [portsStr, name, image] = line.split("\t");
      if (!portsStr || !name) continue;

      const portMatches = portsStr.matchAll(
        /(?:\d+\.\d+\.\d+\.\d+|::|\[::\]):(\d+)->/g,
      );
      const seen = new Set();
      for (const m of portMatches) {
        const port = parseInt(m[1], 10);
        if (!seen.has(port)) {
          seen.add(port);
          map.set(port, { name, image });
        }
      }
    }
  } catch {}
  return map;
}

function getProcessTree(pid) {
  const tree = [];
  try {
    const raw = execSync("ps -eo pid=,ppid=,comm= 2>/dev/null", {
      encoding: "utf8",
      timeout: 5000,
    }).trim();

    const processes = new Map();
    for (const line of raw.split("\n")) {
      const parts = line.trim().split(/\s+/);
      if (parts.length < 3) continue;
      const currentPid = parseInt(parts[0], 10);
      const ppid = parseInt(parts[1], 10);
      processes.set(currentPid, {
        pid: currentPid,
        ppid,
        name: parts.slice(2).join(" "),
      });
    }

    let currentPid = pid;
    let depth = 0;
    while (currentPid > 1 && depth < 8) {
      const proc = processes.get(currentPid);
      if (!proc) break;
      tree.push(proc);
      currentPid = proc.ppid;
      depth++;
    }
  } catch {}

  return tree;
}

export function getListeningPorts(detailed = false) {
  let raw;
  try {
    raw = execSync("lsof -iTCP -sTCP:LISTEN -P -n 2>/dev/null", {
      encoding: "utf8",
      timeout: 10000,
    });
  } catch {
    return [];
  }

  const lines = raw.trim().split("\n").slice(1);
  const portMap = new Map();
  const entries = [];

  for (const line of lines) {
    const parts = line.split(/\s+/);
    if (parts.length < 9) continue;

    const processName = parts[0];
    const pid = parseInt(parts[1], 10);
    const nameField = parts[8];
    const portMatch = nameField.match(/:(\d+)$/);
    if (!portMatch) continue;
    const port = parseInt(portMatch[1], 10);

    if (portMap.has(port)) continue;
    portMap.set(port, true);
    entries.push({ port, pid, processName });
  }

  const uniquePids = [...new Set(entries.map((entry) => entry.pid))];
  const psMap = batchPsInfo(uniquePids);
  const cwdMap = batchCwd(uniquePids);
  const hasDocker = entries.some(
    (entry) =>
      entry.processName.startsWith("com.docke") || entry.processName === "docker",
  );
  const dockerMap = hasDocker ? batchDockerInfo() : new Map();

  return entries
    .map(({ port, pid, processName }) => {
      const ps = psMap.get(pid);
      const cwd = cwdMap.get(pid);
      const docker = dockerMap.get(port);
      const base = {
        port,
        pid,
        processName: docker ? "docker" : processName,
        rawName: processName,
        command: ps ? ps.command : "",
        cwd: docker ? null : cwd,
        executablePath: null,
        framework: docker
          ? detectFrameworkFromImage(docker.image)
          : detectFrameworkFromCommand(ps?.command, processName),
        projectName: docker ? docker.name : null,
        status: "healthy",
        rssKB: ps?.rss ?? null,
        startTime: ps?.lstart ?? null,
        processTree: detailed ? getProcessTree(pid) : [],
      };

      if (ps?.stat?.includes("Z")) {
        base.status = "zombie";
      } else if (ps?.ppid === 1 && isDevProcess(processName, ps.command)) {
        base.status = "orphaned";
      }

      return normalizePortEntry(base, detailed);
    })
    .sort((a, b) => a.port - b.port);
}

export function getPortDetails(targetPort) {
  const ports = getListeningPorts(true);
  return ports.find((port) => port.port === targetPort) || null;
}

export function getAllProcesses() {
  let raw;
  try {
    raw = execSync(
      "ps -eo pid=,pcpu=,pmem=,rss=,lstart=,command= 2>/dev/null",
      { encoding: "utf8", timeout: 5000 },
    ).trim();
  } catch {
    return [];
  }

  const entries = [];
  const seen = new Set();

  for (const line of raw.split("\n")) {
    if (!line.trim()) continue;
    const match = line
      .trim()
      .match(
        /^(\d+)\s+([\d.]+)\s+([\d.]+)\s+(\d+)\s+\w+\s+(\w+\s+\d+\s+[\d:]+\s+\d+)\s+(.*)$/,
      );
    if (!match) continue;

    const pid = parseInt(match[1], 10);
    if (pid <= 1 || pid === process.pid || seen.has(pid)) continue;
    seen.add(pid);

    entries.push({
      pid,
      processName: basename(match[6].split(/\s+/)[0]),
      command: match[6],
      cpu: parseFloat(match[2]),
      rssKB: parseInt(match[4], 10),
      startTime: match[5],
    });
  }

  const nonDocker = entries.filter(
    (entry) =>
      !entry.processName.startsWith("com.docke") &&
      !entry.processName.startsWith("Docker") &&
      entry.processName !== "docker" &&
      entry.processName !== "docker-sandbox",
  );
  const cwdMap = batchCwd(nonDocker.map((entry) => entry.pid));

  return entries.map((entry) =>
    normalizeProcessEntry({
      ...entry,
      cwd: cwdMap.get(entry.pid) || null,
      executablePath: null,
    }),
  );
}

export function findOrphanedProcesses() {
  return getListeningPorts().filter(
    (port) => port.status === "orphaned" || port.status === "zombie",
  );
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
