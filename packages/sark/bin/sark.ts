#!/usr/bin/env bun

import main from '../src/codegen/cli';

const code = await main();
process.exit(code);
