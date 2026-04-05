import { execFileSync } from "child_process";
import { existsSync, readFileSync, statSync } from "fs";
import { join, dirname, basename, isAbsolute, win32 } from "path";

function canonicalProcessName(processName) {
  return (processName || "").toLowerCase().replace(/\.(exe|cmd|bat)$/i, "");
}

export function detectFrameworkFromImage(image) {
  if (!image) return "Docker";
  const img = image.toLowerCase();
  if (img.includes("postgres")) return "PostgreSQL";
  if (img.includes("redis")) return "Redis";
  if (img.includes("mysql") || img.includes("mariadb")) return "MySQL";
  if (img.includes("mongo")) return "MongoDB";
  if (img.includes("nginx")) return "nginx";
  if (img.includes("localstack")) return "LocalStack";
  if (img.includes("rabbitmq")) return "RabbitMQ";
  if (img.includes("kafka")) return "Kafka";
  if (img.includes("elasticsearch") || img.includes("opensearch")) {
    return "Elasticsearch";
  }
  if (img.includes("minio")) return "MinIO";
  return "Docker";
}

export function findProjectRoot(dir) {
  if (!dir) return null;

  const markers = [
    "package.json",
    "Cargo.toml",
    "go.mod",
    "pyproject.toml",
    "Gemfile",
    "pom.xml",
    "build.gradle",
  ];
  const parsedRoot = dirname(dir) === dir ? dir : null;
  let current = dir;
  let depth = 0;

  while (current && current !== parsedRoot && depth < 15) {
    for (const marker of markers) {
      if (existsSync(join(current, marker))) return current;
    }
    const parent = dirname(current);
    if (parent === current) break;
    current = parent;
    depth++;
  }

  return dir;
}

export function detectFramework(projectRoot) {
  if (!projectRoot) return null;

  const pkgPath = join(projectRoot, "package.json");
  if (existsSync(pkgPath)) {
    try {
      const pkg = JSON.parse(readFileSync(pkgPath, "utf8"));
      const allDeps = { ...pkg.dependencies, ...pkg.devDependencies };

      if (allDeps.next) return "Next.js";
      if (allDeps.nuxt || allDeps.nuxt3) return "Nuxt";
      if (allDeps["@sveltejs/kit"]) return "SvelteKit";
      if (allDeps.svelte) return "Svelte";
      if (allDeps["@remix-run/react"] || allDeps.remix) return "Remix";
      if (allDeps.astro) return "Astro";
      if (allDeps.vite) return "Vite";
      if (allDeps["@angular/core"]) return "Angular";
      if (allDeps.vue) return "Vue";
      if (allDeps.react) return "React";
      if (allDeps.express) return "Express";
      if (allDeps.fastify) return "Fastify";
      if (allDeps.hono) return "Hono";
      if (allDeps.koa) return "Koa";
      if (allDeps.nestjs || allDeps["@nestjs/core"]) return "NestJS";
      if (allDeps.gatsby) return "Gatsby";
      if (allDeps["webpack-dev-server"]) return "Webpack";
      if (allDeps.esbuild) return "esbuild";
      if (allDeps.parcel) return "Parcel";
    } catch {}
  }

  if (
    existsSync(join(projectRoot, "vite.config.ts")) ||
    existsSync(join(projectRoot, "vite.config.js"))
  ) {
    return "Vite";
  }
  if (
    existsSync(join(projectRoot, "next.config.js")) ||
    existsSync(join(projectRoot, "next.config.mjs"))
  ) {
    return "Next.js";
  }
  if (existsSync(join(projectRoot, "angular.json"))) return "Angular";
  if (existsSync(join(projectRoot, "Cargo.toml"))) return "Rust";
  if (existsSync(join(projectRoot, "go.mod"))) return "Go";
  if (existsSync(join(projectRoot, "manage.py"))) return "Django";
  if (existsSync(join(projectRoot, "Gemfile"))) return "Ruby";

  return null;
}

export function detectFrameworkFromCommand(command, processName) {
  if (!command) return detectFrameworkFromName(processName);
  const cmd = command.toLowerCase();

  if (cmd.includes("next")) return "Next.js";
  if (cmd.includes("vite")) return "Vite";
  if (cmd.includes("nuxt")) return "Nuxt";
  if (cmd.includes("angular") || cmd.includes("ng serve")) return "Angular";
  if (cmd.includes("webpack")) return "Webpack";
  if (cmd.includes("remix")) return "Remix";
  if (cmd.includes("astro")) return "Astro";
  if (cmd.includes("gatsby")) return "Gatsby";
  if (cmd.includes("flask")) return "Flask";
  if (cmd.includes("django") || cmd.includes("manage.py")) return "Django";
  if (cmd.includes("uvicorn")) return "FastAPI";
  if (cmd.includes("rails")) return "Rails";
  if (cmd.includes("cargo") || cmd.includes("rustc")) return "Rust";

  return detectFrameworkFromName(processName);
}

export function detectFrameworkFromName(processName) {
  const name = canonicalProcessName(processName);
  if (name === "node") return "Node.js";
  if (name === "bun") return "Bun";
  if (name === "python" || name === "python3") return "Python";
  if (name === "ruby") return "Ruby";
  if (name === "java") return "Java";
  if (name === "go") return "Go";
  return null;
}

export function isDevProcess(processName, command) {
  const name = canonicalProcessName(processName);
  const cmd = (command || "").toLowerCase();

  const systemApps = [
    "spotify",
    "raycast",
    "tableplus",
    "postman",
    "linear",
    "cursor",
    "controlce",
    "rapportd",
    "superhuma",
    "setappage",
    "slack",
    "discord",
    "firefox",
    "chrome",
    "google",
    "safari",
    "figma",
    "notion",
    "zoom",
    "teams",
    "code",
    "iterm2",
    "warp",
    "arc",
    "loginwindow",
    "windowserver",
    "systemuise",
    "kernel_task",
    "launchd",
    "mdworker",
    "mds_stores",
    "cfprefsd",
    "coreaudio",
    "corebrightne",
    "airportd",
    "bluetoothd",
    "sharingd",
    "usernoted",
    "notificationc",
    "cloudd",
    "explorer",
    "searchhost",
    "shellexper",
    "taskhostw",
    "dwm",
    "startmenuexperiencehost",
    "widgets",
    "applicationframehost",
  ];
  for (const app of systemApps) {
    if (name.startsWith(app)) return false;
  }

  const devNames = new Set([
    "node",
    "python",
    "python3",
    "ruby",
    "java",
    "go",
    "cargo",
    "deno",
    "bun",
    "php",
    "uvicorn",
    "gunicorn",
    "flask",
    "rails",
    "npm",
    "npx",
    "yarn",
    "pnpm",
    "tsc",
    "tsx",
    "esbuild",
    "rollup",
    "turbo",
    "nx",
    "jest",
    "vitest",
    "mocha",
    "pytest",
    "cypress",
    "playwright",
    "rustc",
    "dotnet",
    "gradle",
    "mvn",
    "mix",
    "elixir",
  ]);
  if (devNames.has(name)) return true;

  if (
    name.startsWith("com.docke") ||
    name.startsWith("docker") ||
    name === "docker-sandbox"
  ) {
    return true;
  }

  const cmdIndicators = [
    /\bnode\b/,
    /\bnext[\s-]/,
    /\bvite\b/,
    /\bnuxt\b/,
    /\bwebpack\b/,
    /\bremix\b/,
    /\bastro\b/,
    /\bgulp\b/,
    /\bng serve\b/,
    /\bgatsb/,
    /\bflask\b/,
    /\bdjango\b|manage\.py/,
    /\buvicorn\b/,
    /\brails\b/,
    /\bcargo\b/,
    /\bdotnet\b/,
  ];
  for (const re of cmdIndicators) {
    if (re.test(cmd)) return true;
  }

  return false;
}

export function summarizeCommand(command, processName) {
  const cmd = command || "";
  const parts = cmd.match(/"[^"]*"|'[^']*'|\S+/g) || [];
  const meaningful = [];
  for (let i = 0; i < parts.length; i++) {
    const part = parts[i].replace(/^['"]|['"]$/g, "");
    if (i === 0 || !part || part.startsWith("-")) continue;
    if (part.includes("/") || part.includes("\\")) {
      meaningful.push(part.includes("\\") ? win32.basename(part) : basename(part));
    } else {
      meaningful.push(part);
    }
    if (meaningful.length >= 3) break;
  }

  if (meaningful.length > 0) return meaningful.join(" ");
  return processName;
}

export function formatUptime(ms) {
  const seconds = Math.floor(ms / 1000);
  const minutes = Math.floor(seconds / 60);
  const hours = Math.floor(minutes / 60);
  const days = Math.floor(hours / 24);

  if (days > 0) return `${days}d ${hours % 24}h`;
  if (hours > 0) return `${hours}h ${minutes % 60}m`;
  if (minutes > 0) return `${minutes}m ${seconds % 60}s`;
  return `${seconds}s`;
}

export function formatMemory(rssKB) {
  if (rssKB > 1048576) return `${(rssKB / 1048576).toFixed(1)} GB`;
  if (rssKB > 1024) return `${(rssKB / 1024).toFixed(1)} MB`;
  return `${Math.round(rssKB)} KB`;
}

export function getGitBranch(cwd) {
  if (!cwd) return null;
  try {
    return execFileSync("git", ["-C", cwd, "rev-parse", "--abbrev-ref", "HEAD"], {
      encoding: "utf8",
      timeout: 3000,
      windowsHide: true,
    }).trim() || null;
  } catch {
    return null;
  }
}

export function normalizePortEntry(base, detailed = false) {
  const info = {
    port: base.port,
    pid: base.pid,
    processName: base.processName,
    rawName: base.rawName || base.processName,
    command: base.command || "",
    cwd: null,
    projectName: base.projectName || null,
    framework: base.framework || null,
    uptime: null,
    startTime: null,
    status: base.status || "healthy",
    memory: null,
    gitBranch: null,
    processTree: detailed ? base.processTree || [] : [],
  };

  if (base.startTime) {
    const startTime = base.startTime instanceof Date
      ? base.startTime
      : new Date(base.startTime);
    if (!Number.isNaN(startTime.getTime())) {
      info.startTime = startTime;
      info.uptime = formatUptime(Date.now() - startTime.getTime());
    }
  }

  if (typeof base.rssKB === "number" && base.rssKB > 0) {
    info.memory = formatMemory(base.rssKB);
  }

  const projectRoot = inferProjectRoot(base.cwd, base.command, base.executablePath);
  if (projectRoot) {
    info.cwd = projectRoot;
    info.projectName = info.projectName || basename(projectRoot);
    info.framework = info.framework || detectFramework(projectRoot);
    if (detailed) {
      info.gitBranch = getGitBranch(projectRoot);
    }
  }

  info.framework =
    info.framework || detectFrameworkFromCommand(info.command, info.processName);

  return info;
}

export function normalizeProcessEntry(base) {
  const info = {
    pid: base.pid,
    processName: base.processName,
    command: base.command || "",
    description: summarizeCommand(base.command, base.processName),
    cpu: typeof base.cpu === "number" ? base.cpu : 0,
    memory: typeof base.rssKB === "number" && base.rssKB > 0
      ? formatMemory(base.rssKB)
      : null,
    cwd: null,
    projectName: null,
    framework: null,
    uptime: null,
  };

  if (base.startTime) {
    const startTime = new Date(base.startTime);
    if (!Number.isNaN(startTime.getTime())) {
      info.uptime = formatUptime(Date.now() - startTime.getTime());
    }
  }

  const projectRoot = inferProjectRoot(base.cwd, base.command, base.executablePath);
  if (projectRoot) {
    info.cwd = projectRoot;
    info.projectName = basename(projectRoot);
    info.framework = detectFramework(projectRoot);
  }

  info.framework =
    info.framework || detectFrameworkFromCommand(info.command, info.processName);

  return info;
}

export function inferProjectRoot(cwd, command, executablePath) {
  const candidates = [];

  if (cwd) candidates.push(cwd);

  const commandPaths = extractPathsFromCommand(command);
  candidates.push(...commandPaths);

  if (executablePath) candidates.push(executablePath);

  for (const candidate of candidates) {
    if (!candidate) continue;
    const normalized = candidate.replaceAll('"', "");
    const dir = isAbsolute(normalized) ? normalized : null;
    if (!dir) continue;
    let baseDir = dirname(dir);
    if (existsSync(dir)) {
      try {
        baseDir = statSync(dir).isDirectory() ? dir : dirname(dir);
      } catch {
        baseDir = dirname(dir);
      }
    }
    if (!baseDir || !existsSync(baseDir)) continue;
    return findProjectRoot(baseDir);
  }

  return null;
}

function extractPathsFromCommand(command) {
  if (!command) return [];
  const matches = command.match(
    /[A-Za-z]:\\(?:[^"\s]+|"[^"]+")+|\/(?:[^"\s]+|"[^"]+")+/g,
  );
  if (!matches) return [];
  return matches.map((match) => match.replaceAll('"', ""));
}
