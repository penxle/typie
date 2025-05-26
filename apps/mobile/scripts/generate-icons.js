import fs from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { exportToDirectory, IconSet } from '@iconify/tools';
import { icons as lucideIcons } from '@iconify-json/lucide';
import { icons as lucideLabIcons } from '@iconify-json/lucide-lab';
import { Case } from 'change-case-all';
import { FontAssetType, generateFonts } from 'fantasticon';
import SVGFixer from 'oslllo-svg-fixer';

const dartKeywords = new Set([
  'abstract',
  'as',
  'assert',
  'async',
  'await',
  'base',
  'break',
  'case',
  'catch',
  'class',
  'const',
  'continue',
  'covariant',
  'default',
  'deferred',
  'do',
  'dynamic',
  'else',
  'enum',
  'export',
  'extends',
  'extension',
  'external',
  'factory',
  'false',
  'final',
  'final',
  'finally',
  'for',
  'Function',
  'get',
  'hide',
  'if',
  'implements',
  'import',
  'in',
  'interface',
  'is',
  'late',
  'library',
  'mixin',
  'new',
  'null',
  'of',
  'on',
  'operator',
  'part',
  'required',
  'rethrow',
  'return',
  'sealed',
  'set',
  'show',
  'static',
  'super',
  'switch',
  'sync',
  'this',
  'throw',
  'true',
  'try',
  'type',
  'typedef',
  'var',
  'void',
  'when',
  'with',
  'while',
  'yield',
]);

const createIconFont = async (name, dir) => {
  await SVGFixer(dir, dir).fix();

  const familyName = `${Case.pascal(name)}Icons`;

  const { codepoints } = await generateFonts({
    inputDir: dir,
    outputDir: '.',
    name: familyName,
    formatOptions: {
      svg: {
        normalize: true,
        fixedWidth: true,
        fontHeight: 1000,
        centerHorizontally: true,
        centerVertically: true,
        preserveAspectRatio: true,
      },
    },
    assetTypes: [],
    fontTypes: [FontAssetType.TTF],
    pathOptions: {
      ttf: `assets/fonts/${familyName}.ttf`,
    },
  });

  const entries = Object.entries(codepoints)
    .reverse()
    .map(([name, codepoint]) => {
      let n = Case.snake(name);
      if (dartKeywords.has(n)) {
        n = `${n}_`;
      }

      return `  static const ${n} = IconData(${codepoint & 0xff_ff}, fontFamily: _fontFamily);`;
    });

  await fs.writeFile(
    `lib/icons/${Case.snake(name)}.dart`,
    `// automatically generated
// ignore_for_file: constant_identifier_names
// spell-checker:disable

import 'package:flutter/material.dart';

@staticIconProvider
abstract final class ${familyName} {
  static const _fontFamily = '${familyName}';
${entries.join('\n')}
}
`,
  );
};

const makeIconSetDir = async (name, icons) => {
  const iconSet = new IconSet(icons);
  iconSet.forEachSync(
    (name) => {
      const svg = iconSet.toSVG(name);
      svg.$svg('[stroke-width="2"]').attr('stroke-width', '1.5');
      iconSet.fromSVG(name, svg);
    },
    ['icon'],
  );

  const dir = await fs.mkdtemp(path.join(os.tmpdir(), `icons-${name}`));
  await exportToDirectory(iconSet, {
    target: dir,
    cleanup: true,
    includeAliases: false,
  });

  return dir;
};

const lucideLightDir = await makeIconSetDir('lucide', lucideIcons);
const lucideLabDir = await makeIconSetDir('lucide-lab', lucideLabIcons);

const typieDir = await fs.mkdtemp(path.join(os.tmpdir(), 'icons-typie-'));
await fs.cp('../website/src/icons', typieDir, { recursive: true });

await createIconFont('lucide-light', lucideLightDir);
await createIconFont('lucide-lab', lucideLabDir);
await createIconFont('typie', typieDir);
