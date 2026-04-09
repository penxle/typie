import fs from 'node:fs/promises';
import path from 'node:path';
import { IconSet, runSVGO, SVG } from '@iconify/tools';
import { icons as lucideIcons } from '@iconify-json/lucide';

const TYPIE_SVG_DIR = path.resolve('..', 'apps', 'website', 'src', 'icons');
const OUTPUT_PATH = path.resolve('icons.json');

const COLORS = new Set(['currentColor', 'none', '']);
const ATTR_RE = /(\w[\w-]*)=["']([^"']*?)["']/g;
const ELEMENT_RE = /<(svg|g|path|circle|rect|line|ellipse|polyline|polygon)\b([^>]*)\/?\s*>/gi;

function parseAttrs(attrStr: string): Record<string, string> {
  const attrs: Record<string, string> = {};
  for (const a of attrStr.matchAll(ATTR_RE)) {
    attrs[a[1]] = a[2];
  }
  return attrs;
}

function resolveAttr(key: string, pathAttrs: Record<string, string>, ...parents: Record<string, string>[]): string {
  if (pathAttrs[key] !== undefined) return pathAttrs[key];
  for (const p of parents) {
    if (p[key] !== undefined) return p[key];
  }
  return '';
}

type IconElement =
  | { style: 'fill'; d: string; fill_rule?: 'evenodd' }
  | { style: 'stroke'; d: string; stroke_cap?: 'round' | 'square'; stroke_join?: 'round' | 'bevel' };

function parseSvgPaths(svgContent: string): IconElement[] {
  const paths: IconElement[] = [];
  let svgAttrs: Record<string, string> = {};
  let gAttrs: Record<string, string> = {};

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
    const hasFill = fill !== '' && fill !== 'none';
    const hasStroke = stroke !== '' && stroke !== 'none';
    const fillRule = attrs['fill-rule'] || attrs['clip-rule'] || gAttrs['fill-rule'] || gAttrs['clip-rule'] || '';
    const lineCap = resolveAttr('stroke-linecap', attrs, gAttrs, svgAttrs);
    const lineJoin = resolveAttr('stroke-linejoin', attrs, gAttrs, svgAttrs);

    const fillElem = (fr: string): IconElement => {
      const e: IconElement = { style: 'fill', d };
      if (fr === 'evenodd') e.fill_rule = 'evenodd';
      return e;
    };
    const strokeElem = (cap: string, join: string): IconElement => {
      const e: IconElement = { style: 'stroke', d };
      if (cap === 'round') e.stroke_cap = 'round';
      else if (cap === 'square') e.stroke_cap = 'square';
      if (join === 'round') e.stroke_join = 'round';
      else if (join === 'bevel') e.stroke_join = 'bevel';
      return e;
    };

    if (hasFill && hasStroke) {
      paths.push(fillElem(fillRule), strokeElem(lineCap, lineJoin));
    } else if (hasStroke) {
      paths.push(strokeElem(lineCap, lineJoin));
    } else if (hasFill) {
      paths.push(fillElem(fillRule));
    } else {
      paths.push(strokeElem(lineCap, lineJoin));
    }
  }

  return paths;
}

function isMultiColor(svgContent: string): boolean {
  for (const match of svgContent.matchAll(ELEMENT_RE)) {
    const attrs = parseAttrs(match[2]);
    for (const key of ['fill', 'stroke']) {
      if (attrs[key] && !COLORS.has(attrs[key])) return true;
    }
  }
  return false;
}

function parseViewBox(svgContent: string): { width: number; height: number } {
  const m = svgContent.match(/viewBox=["']([^"']*)["']/);
  if (!m) return { width: 24, height: 24 };
  const parts = m[1].split(/\s+/).map(Number);
  return { width: parts[2] || 24, height: parts[3] || 24 };
}

const SVGO_SHAPE_TO_PATH = {
  plugins: [
    { name: 'convertShapeToPath', params: { convertArcs: true } },
    {
      name: 'convertRoundedRects',
      fn: () => ({
        element: {
          enter: (node: { name: string; attributes: Record<string, string> }) => {
            if (node.name !== 'rect') return;
            const x = +(node.attributes.x || 0);
            const y = +(node.attributes.y || 0);
            const w = +(node.attributes.width || 0);
            const h = +(node.attributes.height || 0);
            const rx = +(node.attributes.rx || node.attributes.ry || 0);
            const ry = +(node.attributes.ry || rx);
            if (!w || !h) return;

            let d: string;
            if (rx > 0 || ry > 0) {
              const r = Math.min(rx, w / 2);
              const s = Math.min(ry, h / 2);
              d = `M${x + r} ${y}h${w - 2 * r}a${r} ${s} 0 0 1 ${r} ${s}v${h - 2 * s}a${r} ${s} 0 0 1-${r} ${s}h-${w - 2 * r}a${r} ${s} 0 0 1-${r}-${s}v-${h - 2 * s}a${r} ${s} 0 0 1 ${r}-${s}Z`;
            } else {
              d = `M${x} ${y}h${w}v${h}h-${w}Z`;
            }

            node.name = 'path';
            node.attributes.d = d;
            // eslint-disable-next-line @typescript-eslint/no-dynamic-delete
            for (const k of ['x', 'y', 'width', 'height', 'rx', 'ry']) delete node.attributes[k];
          },
        },
      }),
    },
  ],
};

type IconData = {
  viewport: [number, number];
  elements: IconElement[];
};

async function processIcon(svgContent: string): Promise<IconData | null> {
  if (isMultiColor(svgContent)) return null;

  const svg = new SVG(svgContent);
  await runSVGO(svg, SVGO_SHAPE_TO_PATH);
  const processed = svg.toString();

  const elements = parseSvgPaths(processed);
  if (elements.length === 0) return null;

  const { width, height } = parseViewBox(processed);
  return { viewport: [width, height], elements };
}

async function loadLucideIcons(): Promise<Record<string, IconData>> {
  const iconSet = new IconSet(lucideIcons);
  const names: string[] = [];
  iconSet.forEachSync((n) => names.push(n), ['icon']);

  const result: Record<string, IconData> = {};
  for (const name of names) {
    const svg = iconSet.toSVG(name);
    if (!svg) continue;
    const icon = await processIcon(svg.toString());
    if (icon) result[`lucide/${name}`] = icon;
  }
  return result;
}

async function loadTypieIcons(): Promise<Record<string, IconData>> {
  const result: Record<string, IconData> = {};
  let files: string[];
  try {
    const entries = await fs.readdir(TYPIE_SVG_DIR);
    files = entries.toSorted();
  } catch {
    console.warn(`⚠ Typie icon directory not found: ${TYPIE_SVG_DIR}`);
    return result;
  }
  for (const file of files) {
    if (!file.endsWith('.svg')) continue;
    const name = file.replace(/\.svg$/, '');
    const svgContent = await fs.readFile(path.join(TYPIE_SVG_DIR, file), 'utf8');
    const icon = await processIcon(svgContent);
    if (icon) result[`typie/${name}`] = icon;
  }
  return result;
}

const lucide = await loadLucideIcons();
const typie = await loadTypieIcons();
const all = { ...lucide, ...typie };

const sorted = Object.fromEntries(Object.entries(all).toSorted(([a], [b]) => a.localeCompare(b)));

await fs.writeFile(OUTPUT_PATH, JSON.stringify(sorted, null, 2) + '\n');
console.log(`Generated ${Object.keys(sorted).length} icons → ${OUTPUT_PATH}`);
