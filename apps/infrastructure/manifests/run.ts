#!/usr/bin/env bun

import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { $ } from 'bun';
import chalk from 'chalk';

const NODES = [
  ['axolotl', 'worker'],
  ['beaver', 'worker'],
  ['capybara', 'controlplane'],
  ['ferret', 'worker'],
  ['meerkat', 'worker'],
  ['quokka', 'worker'],
  ['wallaby', 'worker'],
];

const args = process.argv.slice(2);
const dryRun = args.includes('--dry-run');
const nodesArg = args.find((_, i) => args[i - 1] === '--nodes');
const selectedNodes = nodesArg ? nodesArg.split(',') : null;

const tempDir = mkdtempSync(path.join(tmpdir(), 'talos-deploy-'));

try {
  await $`sops -d secrets.enc.yaml > ${tempDir}/secrets.yaml`.quiet();
  await $`sops -d patches/secrets.enc.yaml > ${tempDir}/patches-secrets.yaml`.quiet();

  await $`cd ${tempDir} && talosctl gen config typie https://controlplane.k8s.typie.io:6443 --with-secrets secrets.yaml --kubernetes-version 1.34.1`.quiet();

  await Promise.all(
    NODES.map(async ([name, type]) => {
      try {
        const patchCmd =
          type === 'controlplane'
            ? `cd ${tempDir} && talosctl machineconfig patch controlplane.yaml --patch @${process.cwd()}/patches/base.yaml --patch @${process.cwd()}/patches/controlplane.yaml --patch @${process.cwd()}/patches/${name}.yaml --patch @patches-secrets.yaml --output ${name}.yaml`
            : `cd ${tempDir} && talosctl machineconfig patch worker.yaml --patch @${process.cwd()}/patches/base.yaml --patch @${process.cwd()}/patches/${name}.yaml --patch @patches-secrets.yaml --output ${name}.yaml`;

        await $`sh -c ${patchCmd}`.quiet();
        console.log(chalk.green(`${name} patched`));
      } catch {
        console.log(chalk.red(`${name} patch failed`));
        throw new Error(`${name} patch failed`);
      }
    }),
  );

  if (!dryRun) {
    const nodesToDeploy = NODES.filter(([name]) => !selectedNodes || selectedNodes.includes(name));

    await Promise.all(
      nodesToDeploy.map(async ([name]) => {
        try {
          await $`talosctl apply-config --nodes ${name} --file ${tempDir}/${name}.yaml`.quiet();
          console.log(chalk.green(`${name} deployed`));
        } catch {
          console.log(chalk.red(`${name} deployment failed`));
          throw new Error(`${name} deployment failed`);
        }
      }),
    );
  }
} catch (err) {
  console.error(chalk.red(`Error: ${err}`));
  rmSync(tempDir, { recursive: true, force: true });
  process.exit(1);
}

rmSync(tempDir, { recursive: true, force: true });
