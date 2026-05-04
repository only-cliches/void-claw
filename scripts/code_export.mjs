// save as: dump-rs-files.mjs
import { promises as fs } from "node:fs";
import path from "node:path";

async function walk(dir) {
  const entries = await fs.readdir(dir, { withFileTypes: true });
  const results = [];

  for (const entry of entries) {
    const fullPath = path.join(dir, entry.name);

    if (entry.isDirectory()) {
      results.push(...(await walk(fullPath)));
    } else if (entry.isFile() && (entry.name.toLowerCase().endsWith(".rs") || entry.name.toLowerCase().endsWith(".js") || entry.name.toLowerCase().endsWith(".ts") || entry.name.toLowerCase().endsWith(".dockerfile") || entry.name.toLowerCase().endsWith(".py"))) {
      results.push(fullPath);
    }
  }

  return results;
}

function toPosixRelative(base, target) {
  return path.relative(base, target).split(path.sep).join("/");
}

async function main() {
  const cwd = process.cwd();
  const srcDir = path.join(cwd, "src");
  const outputPath = path.join(cwd, "src-files-dump.md");

  let srcStat;
  try {
    srcStat = await fs.stat(srcDir);
  } catch {
    console.error(`Could not find src directory: ${srcDir}`);
    process.exit(1);
  }

  if (!srcStat.isDirectory()) {
    console.error(`Path exists but is not a directory: ${srcDir}`);
    process.exit(1);
  }

  const srcFiles = await walk(srcDir);
  const dockerFiles = await walk("docker");
  const allFiles = [...srcFiles, ...dockerFiles].sort((a, b) => a.localeCompare(b));

  const lines = [];
  
  lines.push("# Harness Hat Source Code");

  if (allFiles.length === 0) {
    lines.push("_No src files found._");
  } else {
    for (const filePath of allFiles) {
      const rel = toPosixRelative(cwd, filePath);
      let content;

      try {
        content = await fs.readFile(filePath, "utf8");
      } catch (err) {
        content = `/* Failed to read file: ${String(err?.message || err)} */`;
      }

      lines.push(`## ${rel}`);
      lines.push("");
      lines.push("```" + filePath.split(".").pop());
      lines.push(content);
      if (!content.endsWith("\n")) lines.push("");
      lines.push("```");
      lines.push("");
    }
  }

  const exampleToml = "harness-hat.example.toml";
  lines.push("## " + exampleToml + "\n```t");
  lines.push((await fs.readFile(exampleToml)).toString());
  lines.push("```\n");

  await fs.writeFile(outputPath, lines.join("\n"), "utf8");
  console.log(`Wrote ${outputPath} (${allFiles.length} src file${allFiles.length === 1 ? "" : "s"})`);
}

main().catch((err) => {
  console.error("Unexpected error:", err);
  process.exit(1);
});
