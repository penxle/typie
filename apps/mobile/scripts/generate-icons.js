import { execFile } from 'node:child_process';
import fs from 'node:fs/promises';
import path from 'node:path';
import { IconSet, runSVGO, SVG } from '@iconify/tools';
import { icons as lucideIcons } from '@iconify-json/lucide';
import { Case } from 'change-case-all';

const kotlinKeywords = new Set([
  'as',
  'break',
  'class',
  'continue',
  'do',
  'else',
  'false',
  'for',
  'fun',
  'if',
  'in',
  'interface',
  'is',
  'null',
  'object',
  'package',
  'return',
  'super',
  'this',
  'throw',
  'true',
  'try',
  'typealias',
  'typeof',
  'val',
  'var',
  'when',
  'while',
]);

const OUTPUT_DIR = 'compose/src/commonMain/kotlin/co/typie/icons';
const TYPIE_SVG_DIR = path.resolve('..', 'website', 'src', 'icons');

const IOS_ROOT = path.resolve('..', 'ios');
const IOS_DESIGN_DIR = path.join(IOS_ROOT, 'Packages', 'Typie', 'Sources', 'Design');
const IOS_ASSETS_DIR = path.join(IOS_DESIGN_DIR, 'Resources', 'Icons.xcassets');
const IOS_GENERATED_DIR = path.join(IOS_DESIGN_DIR, 'Generated');
const IOS_STROKE_WIDTH = 2;

const swiftKeywords = new Set([
  'as',
  'associatedtype',
  'break',
  'case',
  'catch',
  'class',
  'continue',
  'default',
  'defer',
  'deinit',
  'do',
  'else',
  'enum',
  'extension',
  'fallthrough',
  'false',
  'fileprivate',
  'for',
  'func',
  'guard',
  'if',
  'import',
  'in',
  'init',
  'inout',
  'internal',
  'is',
  'let',
  'nil',
  'operator',
  'private',
  'precedencegroup',
  'protocol',
  'public',
  'repeat',
  'rethrows',
  'return',
  'self',
  'static',
  'struct',
  'subscript',
  'super',
  'switch',
  'throw',
  'throws',
  'true',
  'try',
  'typealias',
  'var',
  'where',
  'while',
]);

// ─── SVG Parsing ───

const COLORS = new Set(['currentColor', 'none', '']);
const ATTR_RE = /(\w[\w-]*)=["']([^"']*?)["']/g;
const ELEMENT_RE = /<(svg|g|path|circle|rect|line|ellipse|polyline|polygon)\b([^>]*)\/?\s*>/gi;

function parseAttrs(attrStr) {
  const attrs = {};
  for (const a of attrStr.matchAll(ATTR_RE)) {
    attrs[a[1]] = a[2];
  }
  return attrs;
}

function resolveAttr(key, pathAttrs, ...parents) {
  if (pathAttrs[key] !== undefined) return pathAttrs[key];
  for (const p of parents) {
    if (p[key] !== undefined) return p[key];
  }
  return '';
}

function parseSvgPaths(svgContent) {
  const paths = [];
  let svgAttrs = {};
  let gAttrs = {};

  for (const match of svgContent.matchAll(ELEMENT_RE)) {
    const tag = match[1].toLowerCase();
    const attrs = parseAttrs(match[2]);

    if (tag === 'svg') {
      svgAttrs = attrs;
      continue;
    }
    if (tag === 'g') {
      gAttrs = attrs;
      continue;
    }
    if (tag !== 'path') continue;

    const d = attrs.d;
    if (!d) continue;

    const fill = resolveAttr('fill', attrs, gAttrs, svgAttrs);
    const stroke = resolveAttr('stroke', attrs, gAttrs, svgAttrs);
    const hasFill = fill && fill !== 'none';
    const hasStroke = stroke && stroke !== 'none';
    const fillRule = attrs['fill-rule'] || attrs['clip-rule'] || gAttrs['fill-rule'] || gAttrs['clip-rule'] || '';
    const lineCap = resolveAttr('stroke-linecap', attrs, gAttrs, svgAttrs);
    const lineJoin = resolveAttr('stroke-linejoin', attrs, gAttrs, svgAttrs);

    if (hasFill && hasStroke) {
      paths.push({ d, style: 'Fill', fillRule, lineCap: '', lineJoin: '' }, { d, style: 'Stroke', fillRule: '', lineCap, lineJoin });
    } else if (hasStroke) {
      paths.push({ d, style: 'Stroke', fillRule: '', lineCap, lineJoin });
    } else if (hasFill) {
      paths.push({ d, style: 'Fill', fillRule, lineCap: '', lineJoin: '' });
    } else {
      paths.push({ d, style: 'Stroke', fillRule: '', lineCap, lineJoin });
    }
  }

  return paths;
}

function isMultiColor(svgContent) {
  for (const match of svgContent.matchAll(ELEMENT_RE)) {
    const attrs = parseAttrs(match[2]);
    for (const key of ['fill', 'stroke']) {
      if (attrs[key] && !COLORS.has(attrs[key])) return true;
    }
  }
  return false;
}

function parseViewBox(svgContent) {
  const m = svgContent.match(/viewBox=["']([^"']*)["']/);
  if (!m) return { width: 24, height: 24 };
  const parts = m[1].split(/\s+/).map(Number);
  return { width: parts[2] || 24, height: parts[3] || 24 };
}

// ─── Kotlin Code Generation ───

function toKotlinName(name) {
  let n = Case.pascal(name).replaceAll('_', '');
  if (/^\d/.test(n)) n = `N${n}`;
  if (kotlinKeywords.has(n[0].toLowerCase() + n.slice(1))) n = `${n}_`;
  return n;
}

function escapeKotlin(str) {
  return str
    .replaceAll('\\', '\\\\')
    .replaceAll('"', String.raw`\"`)
    .replaceAll('$', String.raw`\$`);
}

function iconPathToKotlin(p, indent = '        ') {
  const args = [`"${escapeKotlin(p.d)}"`, `PathStyle.${p.style}`];
  if (p.fillRule === 'evenodd') args.push('fillType = PathFillType.EvenOdd');
  if (p.lineCap === 'round') args.push('strokeLineCap = StrokeCap.Round');
  else if (p.lineCap === 'square') args.push('strokeLineCap = StrokeCap.Square');
  if (p.lineJoin === 'round') args.push('strokeLineJoin = StrokeJoin.Round');
  else if (p.lineJoin === 'bevel') args.push('strokeLineJoin = StrokeJoin.Bevel');
  return `${indent}IconPath(${args.join(', ')})`;
}

const CHUNK_SIZE = 200;

function generateIconEntry(icon, indent = '    ') {
  const pathLines = icon.paths.map((p) => iconPathToKotlin(p)).join(',\n');
  const viewportArgs =
    icon.viewportWidth !== 24 || icon.viewportHeight !== 24
      ? `, viewportWidth = ${icon.viewportWidth}f, viewportHeight = ${icon.viewportHeight}f`
      : '';
  return `${indent}val ${icon.name} = IconData(listOf(\n${pathLines},\n${indent})${viewportArgs})`;
}

function generateKotlin(objectName, icons) {
  const sorted = icons.toSorted((a, b) => a.name.localeCompare(b.name, 'en'));

  const header = `// automatically generated — do not edit
// spell-checker:disable
@file:Suppress("ktlint")

package co.typie.icons

import androidx.compose.ui.graphics.PathFillType
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.StrokeJoin
import co.typie.ui.icon.IconData
import co.typie.ui.icon.IconPath
import co.typie.ui.icon.PathStyle
`;

  // Small icon sets: no chunking needed
  if (sorted.length <= CHUNK_SIZE) {
    const entries = sorted.map((icon) => generateIconEntry(icon));
    return `${header}
object ${objectName} {
${entries.join('\n')}
}
`;
  }

  // Large icon sets: split into private chunk objects to avoid <clinit> 64KB limit
  const chunks = [];
  for (let i = 0; i < sorted.length; i += CHUNK_SIZE) {
    chunks.push(sorted.slice(i, i + CHUNK_SIZE));
  }

  const chunkObjects = chunks
    .map((chunk, i) => {
      const entries = chunk.map((icon) => generateIconEntry(icon));
      return `private object ${objectName}${i} {\n${entries.join('\n')}\n}`;
    })
    .join('\n\n');

  const facade = chunks
    .flatMap((chunk, i) => chunk.map((icon) => `    val ${icon.name}: IconData get() = ${objectName}${i}.${icon.name}`))
    .join('\n');

  return `${header}
${chunkObjects}

object ${objectName} {
${facade}
}
`;
}

// ─── SVG Processing ───

const SVGO_SHAPE_TO_PATH = {
  plugins: [
    { name: 'convertShapeToPath', params: { convertArcs: true } },
    // convertShapeToPath skips rounded rects — handle manually
    {
      name: 'convertRoundedRects',
      fn: () => ({
        element: {
          enter: (node) => {
            if (node.name !== 'rect') return;
            const x = +(node.attributes.x || 0);
            const y = +(node.attributes.y || 0);
            const w = +(node.attributes.width || 0);
            const h = +(node.attributes.height || 0);
            const rx = +(node.attributes.rx || node.attributes.ry || 0);
            const ry = +(node.attributes.ry || rx);
            if (!w || !h) return;

            let d;
            if (rx > 0 || ry > 0) {
              const r = Math.min(rx, w / 2);
              const s = Math.min(ry, h / 2);
              d = `M${x + r} ${y}h${w - 2 * r}a${r} ${s} 0 0 1 ${r} ${s}v${h - 2 * s}a${r} ${s} 0 0 1-${r} ${s}h-${w - 2 * r}a${r} ${s} 0 0 1-${r}-${s}v-${h - 2 * s}a${r} ${s} 0 0 1 ${r}-${s}Z`;
            } else {
              d = `M${x} ${y}h${w}v${h}h-${w}Z`;
            }

            node.name = 'path';
            node.attributes.d = d;
            delete node.attributes.x;
            delete node.attributes.y;
            delete node.attributes.width;
            delete node.attributes.height;
            delete node.attributes.rx;
            delete node.attributes.ry;
          },
        },
      }),
    },
  ],
};

async function processIcon(name, svgContent) {
  if (isMultiColor(svgContent)) {
    console.warn(`⚠ Skipping multi-color icon: ${name}`);
    return null;
  }

  const svg = new SVG(svgContent);
  await runSVGO(svg, SVGO_SHAPE_TO_PATH);
  const processed = svg.toString();

  const paths = parseSvgPaths(processed);
  if (paths.length === 0) {
    console.warn(`⚠ No paths found in: ${name}`);
    return null;
  }

  const { width, height } = parseViewBox(processed);
  return {
    name: toKotlinName(name),
    rawName: name,
    svg: processed,
    paths,
    viewportWidth: width,
    viewportHeight: height,
  };
}

// ─── Icon Sources ───

async function loadLucideIcons() {
  const iconSet = new IconSet(lucideIcons);
  const names = [];
  iconSet.forEachSync((n) => names.push(n), ['icon']);

  const icons = [];
  for (const name of names) {
    const svg = iconSet.toSVG(name);
    if (!svg) continue;
    const icon = await processIcon(name, svg.toString());
    if (icon) icons.push(icon);
  }
  return icons;
}

async function loadTypieIcons() {
  const allFiles = await fs.readdir(TYPIE_SVG_DIR);
  const files = allFiles.toSorted();
  const icons = [];
  for (const file of files) {
    if (!file.endsWith('.svg')) continue;
    const svgContent = await fs.readFile(path.join(TYPIE_SVG_DIR, file), 'utf8');
    const icon = await processIcon(path.parse(file).name, svgContent);
    if (icon) icons.push(icon);
  }
  return icons;
}

// ─── iOS Generation ───

function toSwiftName(name) {
  let n = Case.camel(name).replaceAll('_', '');
  if (/^\d/.test(n)) n = `n${n}`;
  if (swiftKeywords.has(n)) n = `\`${n}\``;
  return n;
}

async function collectIosIconReferences() {
  const refs = { LucideIcon: new Set(), TypieIcon: new Set() };
  const skip = new Set(['.build', '.swiftpm', 'build', 'DerivedData', 'xcuserdata', 'node_modules', 'Generated']);
  async function walk(dir) {
    for (const entry of await fs.readdir(dir, { withFileTypes: true })) {
      if (skip.has(entry.name)) continue;
      const full = path.join(dir, entry.name);
      if (entry.isDirectory()) await walk(full);
      else if (entry.name.endsWith('.swift')) {
        const source = await fs.readFile(full, 'utf8');
        for (const match of source.matchAll(/\b(LucideIcon|TypieIcon)\.(`?[A-Za-z0-9_]+`?)/g)) refs[match[1]].add(match[2]);
      }
    }
  }
  await walk(IOS_ROOT);
  return refs;
}

function selectReferenced(sets) {
  const selected = {};
  const missing = [];
  for (const [label, { icons, referenced }] of Object.entries(sets)) {
    const bySwiftName = new Map(icons.map((icon) => [toSwiftName(icon.rawName), icon]));
    for (const name of referenced) {
      if (!bySwiftName.has(name)) missing.push(`${label}.${name}`);
    }
    selected[label] = [...referenced].filter((name) => bySwiftName.has(name)).map((name) => bySwiftName.get(name));
  }
  if (missing.length > 0) {
    console.error(`✖ referenced icons not found: ${missing.join(', ')}`);
    process.exit(1);
  }
  return selected;
}

function iosSvg(svg) {
  return svg.replaceAll(/stroke-width="[^"]*"/g, `stroke-width="${IOS_STROKE_WIDTH}"`);
}

async function writeIosCatalog(namespace, icons) {
  const dir = path.join(IOS_ASSETS_DIR, namespace);
  await fs.rm(dir, { recursive: true, force: true });
  await fs.mkdir(dir, { recursive: true });
  await fs.writeFile(
    path.join(dir, 'Contents.json'),
    JSON.stringify({ info: { author: 'xcode', version: 1 }, properties: { 'provides-namespace': true } }, null, 2) + '\n',
  );
  for (const icon of icons) {
    const set = path.join(dir, `${icon.rawName}.imageset`);
    await fs.mkdir(set, { recursive: true });
    await fs.writeFile(path.join(set, `${icon.rawName}.svg`), iosSvg(icon.svg) + '\n');
    await fs.writeFile(
      path.join(set, 'Contents.json'),
      JSON.stringify(
        {
          images: [{ filename: `${icon.rawName}.svg`, idiom: 'universal' }],
          info: { author: 'xcode', version: 1 },
          properties: { 'preserves-vector-representation': true, 'template-rendering-intent': 'template' },
        },
        null,
        2,
      ) + '\n',
    );
  }
}

function generateSwift(enumName, namespace, icons) {
  const sorted = icons.toSorted((a, b) => a.rawName.localeCompare(b.rawName, 'en'));
  const names = sorted.map((icon) => toSwiftName(icon.rawName));
  if (new Set(names).size !== names.length) {
    console.error(`✖ ${enumName}: duplicate Swift names`);
    process.exit(1);
  }
  const entries = sorted.map((icon, i) => `  public static let ${names[i]} = TIconName("${namespace}/${icon.rawName}")`);
  return `// automatically generated — do not edit\n// spell-checker:disable\n\npublic enum ${enumName} {\n${entries.join('\n')}\n}\n`;
}

function formatSwift(content, filepath) {
  return new Promise((resolve, reject) => {
    const child = execFile('swift', ['format', '--assume-filename', filepath, '-'], (error, stdout) =>
      error ? reject(error) : resolve(stdout),
    );
    child.stdin.end(content);
  });
}

async function writeSwift(enumName, namespace, icons) {
  const filepath = path.join(IOS_GENERATED_DIR, `${enumName}.swift`);
  await fs.writeFile(filepath, await formatSwift(generateSwift(enumName, namespace, icons), filepath));
}

// ─── Main ───

const lucideIcons2 = await loadLucideIcons();
const typieIcons = await loadTypieIcons();

console.log(`Lucide: ${lucideIcons2.length} icons`);
console.log(`Typie: ${typieIcons.length} icons`);

const iosRefs = await collectIosIconReferences();
const { LucideIcon: iosLucide, TypieIcon: iosTypie } = selectReferenced({
  LucideIcon: { icons: lucideIcons2, referenced: iosRefs.LucideIcon },
  TypieIcon: { icons: typieIcons, referenced: iosRefs.TypieIcon },
});
console.log(`iOS Lucide: ${iosLucide.length} referenced icons`);
console.log(`iOS Typie: ${iosTypie.length} referenced icons`);

await fs.mkdir(OUTPUT_DIR, { recursive: true });
await fs.writeFile(path.join(OUTPUT_DIR, 'Lucide.kt'), generateKotlin('Lucide', lucideIcons2));
await fs.writeFile(path.join(OUTPUT_DIR, 'Typie.kt'), generateKotlin('Typie', typieIcons));

await fs.mkdir(IOS_ASSETS_DIR, { recursive: true });
await fs.writeFile(path.join(IOS_ASSETS_DIR, 'Contents.json'), JSON.stringify({ info: { author: 'xcode', version: 1 } }, null, 2) + '\n');
await writeIosCatalog('lucide', iosLucide);
await writeIosCatalog('typie', iosTypie);
await fs.mkdir(IOS_GENERATED_DIR, { recursive: true });
await writeSwift('LucideIcon', 'lucide', iosLucide);
await writeSwift('TypieIcon', 'typie', iosTypie);

console.log('Done!');
