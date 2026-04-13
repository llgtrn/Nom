const fs = require("node:fs");
const { spawn } = require("node:child_process");
const os = require("node:os");
const path = require("node:path");

const defaultCli =
  "C:\\Users\\trngh\\Documents\\APP\\GitNexus-main\\gitnexus\\dist\\cli\\index.js";

const candidates = [process.env.GITNEXUS_CLI, defaultCli].filter(Boolean);
const cliPath = candidates.find((candidate) => fs.existsSync(candidate));

function loadGitNexusConfig() {
  const configPath = path.join(os.homedir(), ".gitnexus", "config.json");
  try {
    if (!fs.existsSync(configPath)) {
      return {};
    }
    return JSON.parse(fs.readFileSync(configPath, "utf8"));
  } catch {
    return {};
  }
}

if (!cliPath) {
  console.error("GitNexus CLI not found.");
  console.error("Set GITNEXUS_CLI to a local gitnexus dist/cli/index.js path.");
  console.error(`Checked fallback: ${defaultCli}`);
  process.exit(1);
}

const passthroughArgs = process.argv.slice(2);
const childArgs = [cliPath, ...(passthroughArgs.length ? passthroughArgs : ["mcp"])];
const savedConfig = loadGitNexusConfig();
const childEnv = { ...process.env };

if (!childEnv.GITNEXUS_API_KEY && savedConfig.apiKey) {
  childEnv.GITNEXUS_API_KEY = savedConfig.apiKey;
}
if (!childEnv.GITNEXUS_LLM_BASE_URL && savedConfig.baseUrl) {
  childEnv.GITNEXUS_LLM_BASE_URL = savedConfig.baseUrl;
}
if (!childEnv.GITNEXUS_MODEL && savedConfig.model) {
  childEnv.GITNEXUS_MODEL = savedConfig.model;
}

const child = spawn(process.execPath, childArgs, {
  stdio: "inherit",
  env: childEnv,
});

child.on("error", (error) => {
  console.error(`Failed to start GitNexus MCP: ${error.message}`);
  process.exit(1);
});

child.on("exit", (code) => {
  process.exit(code ?? 0);
});
