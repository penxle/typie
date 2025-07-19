#!/usr/bin/env node

import { readFileSync } from 'node:fs';
import path from 'node:path';
import { GetParametersByPathCommand, SSMClient } from '@aws-sdk/client-ssm';
import { program } from 'commander';
import { execa } from 'execa';

type PackageJson = {
  name?: string;
};

async function getProjectName(): Promise<string> {
  let currentDir = process.cwd();

  while (currentDir !== path.dirname(currentDir)) {
    try {
      const packageJsonPath = path.join(currentDir, 'package.json');
      const packageJson: PackageJson = JSON.parse(readFileSync(packageJsonPath, 'utf8'));

      if (packageJson.name) {
        const match = packageJson.name.match(/@typie\/(.+)/);
        if (match) {
          return match[1];
        }
      }
    } catch {
      // pass
    }

    currentDir = path.dirname(currentDir);
  }

  console.error('Error: Could not find a package.json with @typie/* name in current or parent directories');
  process.exit(1);
}

async function getParametersFromSSM(projectName: string, stage: string): Promise<Record<string, string>> {
  const client = new SSMClient({});
  const prefix = `/apps/${projectName}/${stage}/`;

  try {
    const parameters: Record<string, string> = {};
    let nextToken: string | undefined;

    do {
      const command = new GetParametersByPathCommand({
        Path: prefix,
        Recursive: false,
        WithDecryption: true,
        NextToken: nextToken,
      });

      const response = await client.send(command);

      if (response.Parameters) {
        for (const param of response.Parameters) {
          if (param.Name && param.Value) {
            const envName = param.Name.replace(prefix, '');
            parameters[envName] = param.Value;
          }
        }
      }

      nextToken = response.NextToken;
    } while (nextToken);

    return parameters;
  } catch (err) {
    console.error(`Error fetching parameters from AWS SSM for prefix ${prefix}:`, err);
    return {};
  }
}

async function main() {
  program
    .name('rps')
    .description('Run commands with AWS Parameter Store environment variables')
    .version('1.0.0')
    .option('-s, --stage <stage>', 'Stage name', 'local')
    .argument('[command...]', 'Command to run')
    .action(async (commandArgs: string[], options) => {
      if (commandArgs.length === 0) {
        console.error('Error: No command specified');
        console.error('Usage: rps [-s stage] -- <command>');
        process.exit(1);
      }

      const projectName = await getProjectName();

      const parameters = await getParametersFromSSM(projectName, options.stage);

      const env = {
        ...process.env,
        ...parameters,
      };

      const [command, ...args] = commandArgs;

      try {
        await execa(command, args, {
          env,
          stdio: 'inherit',
          shell: true,
        });
      } catch (err) {
        const exitCode = err && typeof err === 'object' && 'exitCode' in err ? err.exitCode : 1;
        process.exit(exitCode as number);
      }
    });

  program.parse();
}

try {
  await main();
} catch (err) {
  console.error('Unexpected error:', err);
  process.exit(1);
}
