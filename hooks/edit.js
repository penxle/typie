#!/usr/bin/env bun

const run = async (cmd) => {
  const p = Bun.spawn({
    cmd,
    stdout: 2,
    stderr: 2,
  });

  const code = await p.exited;

  if (code !== 0) {
    process.exit(2);
  }
};

try {
  const hookData = await Bun.stdin.json();

  const filePath = hookData.tool_input.file_path;
  if (!filePath) {
    process.exit(0);
  }

  if (/\.(ts|tsx|js|jsx|svelte)$/.test(filePath)) {
    await run(['bun', 'eslint', '--fix', filePath]);
  }

  if (/\.(dart)$/.test(filePath)) {
    await run(['dart', 'fix', '--apply', filePath]);
    await run(['dart', 'format', filePath]);
  }

  await run(['bun', 'prettier', '--write', filePath]);

  process.exit(0);
} catch (err) {
  console.error(err);
  process.exit(1);
}
