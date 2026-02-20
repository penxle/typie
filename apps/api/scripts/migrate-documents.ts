#!/usr/bin/env tsx

import { and, eq, sql } from 'drizzle-orm';
import { LoroDoc, LoroMap, LoroText } from 'loro-crdt';
import { DEFAULT_FONT_FAMILIES } from '@/const';
import { db, DocumentContents, Documents, Entities, FontFamilies, Fonts, pg } from '@/db';
import { DocumentSyncType } from '@/enums';
import { pubsub } from '@/pubsub';
import { wasm } from '@/utils/wasm';

process.env.SCRIPT = 'true';

const force = process.argv.includes('--force');

type Delta = {
  insert: string;
  attributes?: Record<string, unknown>;
};

type NewStyle =
  | { type: 'font_weight'; weight: number }
  | { type: 'bold' }
  | { type: 'italic' }
  | { type: 'strikethrough' }
  | { type: 'underline' }
  | { type: 'text_color'; color: string }
  | { type: 'background_color'; color: string }
  | { type: 'font_family'; family: string }
  | { type: 'font_size'; size: number }
  | { type: 'letter_spacing'; spacing: number };

type NewAnnotation = { type: 'link'; href: string } | { type: 'ruby'; text: string };

type NewTextSegment = {
  text: string;
  styles?: NewStyle[];
  annotations?: NewAnnotation[];
};

function convertAttribute(key: string, value: unknown): { style?: NewStyle; annotation?: NewAnnotation } | null {
  if (value === null || value === undefined) return null;

  switch (key) {
    case 'font_weight': {
      return { style: { type: 'font_weight', weight: value as number } };
    }
    case 'italic': {
      return { style: { type: 'italic' } };
    }
    case 'strikethrough': {
      return { style: { type: 'strikethrough' } };
    }
    case 'underline': {
      return { style: { type: 'underline' } };
    }
    case 'text_color': {
      return { style: { type: 'text_color', color: value as string } };
    }
    case 'background_color': {
      return { style: { type: 'background_color', color: value as string } };
    }
    case 'font_family': {
      return { style: { type: 'font_family', family: value as string } };
    }
    case 'font_size': {
      return { style: { type: 'font_size', size: value as number } };
    }
    case 'letter_spacing': {
      return { style: { type: 'letter_spacing', spacing: value as number } };
    }
    case 'link': {
      return { annotation: { type: 'link', href: value as string } };
    }
    case 'ruby': {
      return { annotation: { type: 'ruby', text: value as string } };
    }
    default: {
      return null;
    }
  }
}

const DEFAULT_STYLES: NewStyle[] = [
  { type: 'font_family', family: 'Pretendard' },
  { type: 'font_size', size: 1200 },
  { type: 'font_weight', weight: 400 },
  { type: 'text_color', color: 'black' },
  { type: 'background_color', color: 'none' },
  { type: 'letter_spacing', spacing: 0 },
];

function fillDefaultStyles(styles: NewStyle[]): NewStyle[] {
  const presentTypes = new Set(styles.map((s) => s.type));
  const filled = [...styles];
  for (const def of DEFAULT_STYLES) {
    if (!presentTypes.has(def.type)) {
      filled.push(def);
    }
  }
  return filled;
}

function convertDeltaToSegments(delta: Delta[], fillDefaults = true): NewTextSegment[] {
  return delta.map((d) => {
    const styles: NewStyle[] = [];
    const annotations: NewAnnotation[] = [];

    if (d.attributes) {
      for (const [key, value] of Object.entries(d.attributes)) {
        if (value === null) continue;

        const converted = convertAttribute(key, value);
        if (converted?.style) styles.push(converted.style);
        if (converted?.annotation) annotations.push(converted.annotation);
      }
    }

    const segment: NewTextSegment = { text: d.insert, styles: fillDefaults ? fillDefaultStyles(styles) : styles };
    if (annotations.length > 0) segment.annotations = annotations;

    return segment;
  });
}

const BLOCKQUOTE_VARIANT_MAP: Record<string, string> = {
  'left-line': 'left_line',
  'left-quote': 'left_quote',
  'message-sent': 'message_sent',
  'message-received': 'message_received',
};

const HORIZONTAL_RULE_VARIANT_MAP: Record<string, string> = {
  'light-line': 'line',
  'dashed-line': 'dashed_line',
  'circle-line': 'circle_line',
  'diamond-line': 'diamond_line',
  'three-circles': 'three_circles',
  'three-diamonds': 'three_diamonds',
};

function normalizeNode(nodeJson: Record<string, unknown>): Record<string, unknown> {
  const type = nodeJson.type as string;
  const variant = nodeJson.variant as string | undefined;
  if (!variant) return nodeJson;

  if (type === 'blockquote' && variant in BLOCKQUOTE_VARIANT_MAP) {
    return { ...nodeJson, variant: BLOCKQUOTE_VARIANT_MAP[variant] };
  }

  if (type === 'horizontal_rule' && variant in HORIZONTAL_RULE_VARIANT_MAP) {
    return { ...nodeJson, variant: HORIZONTAL_RULE_VARIANT_MAP[variant] };
  }

  return nodeJson;
}

const generateNodeId = () => crypto.randomUUID().replaceAll('-', '');
const DEFAULT_TABLE_CELL_WIDTH_PX = 80;

function isFiniteNumber(value: unknown): value is number {
  return typeof value === 'number' && Number.isFinite(value);
}

function findClosestWeight(target: number, weights: number[]): number {
  let closest = weights[0];
  let minDist = Math.abs(target - closest);
  for (const w of weights) {
    const dist = Math.abs(target - w);
    if (dist < minDist || (dist === minDist && w > closest)) {
      closest = w;
      minDist = dist;
    }
  }
  return closest;
}

const weightCache = new Map<string, number[]>();

async function getAvailableWeights(familyName: string, userId: string): Promise<number[]> {
  const cacheKey = `${userId}:${familyName}`;
  const cached = weightCache.get(cacheKey);
  if (cached) return cached;

  const defaultFamily = DEFAULT_FONT_FAMILIES.find((f) => f.familyName === familyName);
  if (defaultFamily) {
    const weights = defaultFamily.fonts.map((f) => f.weight);
    weightCache.set(cacheKey, weights);
    return weights;
  }

  const fonts = await db
    .select({ weight: Fonts.weight })
    .from(Fonts)
    .innerJoin(FontFamilies, eq(Fonts.familyId, FontFamilies.id))
    .where(and(eq(FontFamilies.familyName, familyName), eq(FontFamilies.userId, userId)));
  const weights = [...new Set(fonts.map((f) => f.weight))];
  weightCache.set(cacheKey, weights);
  return weights;
}

async function validateFontStyles(styles: NewStyle[], userId: string): Promise<NewStyle[]> {
  const familyStyleIndex = styles.findIndex((s) => s.type === 'font_family');
  if (familyStyleIndex === -1) return styles;

  const familyStyle = styles[familyStyleIndex] as { type: 'font_family'; family: string };
  const availableWeights = await getAvailableWeights(familyStyle.family, userId);

  if (availableWeights.length === 0) {
    const result = [...styles];
    result[familyStyleIndex] = { type: 'font_family', family: 'Pretendard' };
    return result;
  }

  const weightStyleIndex = styles.findIndex((s) => s.type === 'font_weight');
  const currentWeight = weightStyleIndex === -1 ? 400 : (styles[weightStyleIndex] as { type: 'font_weight'; weight: number }).weight;

  if (availableWeights.includes(currentWeight)) return styles;

  const result = [...styles];
  let newWeight: number;
  let addBold = false;

  if (currentWeight >= 700 && availableWeights.length === 1) {
    newWeight = availableWeights[0];
    addBold = true;
  } else if (currentWeight >= 700) {
    newWeight = findClosestWeight(700, availableWeights);
  } else {
    newWeight = findClosestWeight(currentWeight, availableWeights);
  }

  if (weightStyleIndex === -1) {
    result.push({ type: 'font_weight', weight: newWeight });
  } else {
    result[weightStyleIndex] = { type: 'font_weight', weight: newWeight };
  }

  if (addBold) {
    result.push({ type: 'bold' });
  }

  return result;
}

type ColWidthMigrationResult = {
  changed: boolean;
  migratedTableCount: number;
  skippedMixedTableCount: number;
};

function migrateTableColWidths(nodes: Record<string, Record<string, unknown>>): ColWidthMigrationResult {
  let changed = false;
  let migratedTableCount = 0;
  let skippedMixedTableCount = 0;

  for (const tableNode of Object.values(nodes)) {
    if (tableNode.type !== 'table' || !Array.isArray(tableNode.children) || tableNode.children.length === 0) {
      continue;
    }

    const firstRowId = tableNode.children[0];
    if (typeof firstRowId !== 'string') {
      continue;
    }

    const firstRowNode = nodes[firstRowId];
    if (!firstRowNode || firstRowNode.type !== 'table_row' || !Array.isArray(firstRowNode.children)) {
      continue;
    }

    const firstRowCells = firstRowNode.children
      .map((cellId) => (typeof cellId === 'string' ? nodes[cellId] : undefined))
      .filter((cell): cell is Record<string, unknown> => !!cell && cell.type === 'table_cell');

    if (firstRowCells.length === 0) {
      continue;
    }

    let hasLegacyPx = false;
    let hasAlreadyMigratedRatio = false;

    for (const cellNode of firstRowCells) {
      const colWidth = cellNode.col_width;
      if (!isFiniteNumber(colWidth)) {
        continue;
      }

      if (colWidth > 1) {
        hasLegacyPx = true;
      } else {
        hasAlreadyMigratedRatio = true;
      }
    }

    if (!hasLegacyPx) {
      continue;
    }

    if (hasAlreadyMigratedRatio) {
      skippedMixedTableCount++;
      continue;
    }

    const widthsPx = firstRowCells.map((cellNode) => {
      const colWidth = cellNode.col_width;
      if (isFiniteNumber(colWidth) && colWidth > 1) {
        return colWidth;
      }
      return DEFAULT_TABLE_CELL_WIDTH_PX;
    });

    const totalWidthPx = widthsPx.reduce((sum, width) => sum + width, 0);
    if (totalWidthPx <= 0) {
      continue;
    }

    for (const [index, cellNode] of firstRowCells.entries()) {
      const migratedWidth = widthsPx[index] / totalWidthPx;
      if (cellNode.col_width !== migratedWidth) {
        cellNode.col_width = migratedWidth;
        changed = true;
      }
    }

    migratedTableCount++;
  }

  return {
    changed,
    migratedTableCount,
    skippedMixedTableCount,
  };
}

function migrateStyleValues(nodes: Record<string, Record<string, unknown>>): boolean {
  let changed = false;
  for (const node of Object.values(nodes)) {
    if (node.type !== 'text' || !Array.isArray(node.text)) continue;
    for (const seg of node.text as NewTextSegment[]) {
      if (!seg.styles) continue;
      for (const style of seg.styles) {
        if (style.type === 'font_size') {
          const s = style as { type: 'font_size'; size: number };
          if (s.size < 500 && s.size > 0) {
            s.size = Math.round(s.size * 100);
            changed = true;
          }
        }
        if (style.type === 'letter_spacing') {
          const s = style as { type: 'letter_spacing'; spacing: number };
          if (Math.abs(s.spacing) < 5 && s.spacing !== 0) {
            s.spacing = Math.round(s.spacing * 100);
            changed = true;
          }
        }
      }
    }
  }
  return changed;
}

function migrateNodeAttrs(nodes: Record<string, Record<string, unknown>>): boolean {
  let changed = false;
  for (const node of Object.values(nodes)) {
    if (node.type === 'paragraph' || node.type === 'fold_title') {
      const lh = node.line_height;
      if (typeof lh === 'number' && lh < 10 && lh !== 0) {
        node.line_height = Math.round(lh * 100);
        changed = true;
      }
    }
  }
  return changed;
}

function migrateSettings(settings: Record<string, unknown>): boolean {
  let changed = false;
  if (typeof settings.block_gap === 'number' && settings.block_gap < 10 && settings.block_gap !== 0) {
    settings.block_gap = Math.round(settings.block_gap * 100);
    changed = true;
  }
  if (typeof settings.paragraph_indent === 'number' && settings.paragraph_indent < 10 && settings.paragraph_indent !== 0) {
    settings.paragraph_indent = Math.round(settings.paragraph_indent * 100);
    changed = true;
  }
  return changed;
}

function fixTextNewlines(nodes: Record<string, Record<string, unknown>>): boolean {
  let fixed = false;
  const replacements = new Map<string, string[]>();

  for (const [nodeId, node] of Object.entries(nodes)) {
    if (node.type !== 'text' || !Array.isArray(node.text)) continue;

    const segments = node.text as NewTextSegment[];
    if (!segments.some((s) => s.text.includes('\n'))) continue;

    const parentId = node.parent as string;
    const groups: NewTextSegment[][] = [[]];

    for (const segment of segments) {
      const parts = segment.text.split('\n');

      for (const [i, part] of parts.entries()) {
        if (i > 0) {
          groups.push([]);
        }
        if (part.length > 0) {
          const newSeg: NewTextSegment = { text: part, styles: segment.styles };
          if (segment.annotations && segment.annotations.length > 0) {
            newSeg.annotations = segment.annotations;
          }
          // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
          groups.at(-1)!.push(newSeg);
        }
      }
    }

    const newChildIds: string[] = [];

    for (const [i, group] of groups.entries()) {
      if (i > 0) {
        const hardBreakId = generateNodeId();
        nodes[hardBreakId] = { type: 'hard_break', children: [], parent: parentId };
        newChildIds.push(hardBreakId);
      }

      if (group.length > 0) {
        const newTextId = i === 0 ? nodeId : generateNodeId();
        nodes[newTextId] = { type: 'text', text: group, children: [], parent: parentId };
        newChildIds.push(newTextId);
      }
    }

    if (!newChildIds.includes(nodeId)) {
      // eslint-disable-next-line @typescript-eslint/no-dynamic-delete
      delete nodes[nodeId];
    }

    replacements.set(nodeId, newChildIds);
    fixed = true;
  }

  for (const node of Object.values(nodes)) {
    if (!Array.isArray(node.children)) continue;
    node.children = (node.children as string[]).flatMap((childId) => replacements.get(childId) ?? [childId]);
  }

  return fixed;
}

function isAlreadyMigrated(doc: LoroDoc): boolean {
  const stylesMap = doc.getMap('styles');
  if (stylesMap.size > 0) return true;

  try {
    const nodesMap = doc.getMap('nodes');
    const rootMap = nodesMap.get('00000000000000000000000000000000') as LoroMap | undefined;
    if (rootMap && rootMap.get('cascade_attrs') != null) return true;
  } catch {
    // ignore
  }

  return false;
}

const CONCURRENCY = 10;

await (async () => {
  if (force) {
    console.log('Force mode enabled — re-migrating all documents');
  }

  console.log(`Starting mark → style/annotation migration... (concurrency: ${CONCURRENCY})`);

  const ids = await db
    .select({
      id: DocumentContents.id,
      documentId: DocumentContents.documentId,
      userId: Entities.userId,
    })
    .from(DocumentContents)
    .innerJoin(Documents, eq(DocumentContents.documentId, Documents.id))
    .innerJoin(Entities, eq(Documents.entityId, Entities.id));

  console.log(`Found ${ids.length} documents to process`);

  let migrated = 0;
  let skipped = 0;
  let errors = 0;
  let migratedTables = 0;
  let skippedMixedTables = 0;

  async function migrateDocument({ id, documentId, userId }: { id: string; documentId: string; userId: string }) {
    try {
      const [row] = await db.select({ snapshot: DocumentContents.snapshot }).from(DocumentContents).where(eq(DocumentContents.id, id));

      if (!row) {
        skipped++;
        return;
      }

      const doc = new LoroDoc();
      doc.import(row.snapshot);

      const allNodesMap = doc.getMap('nodes');
      if (allNodesMap.size === 0) {
        skipped++;
        return;
      }

      // Fix table nodes missing proportion field (added in a later migration)
      for (const nodeId of allNodesMap.keys()) {
        const nodeMap = allNodesMap.get(nodeId) as LoroMap;
        if ((nodeMap.get('type') as string) === 'table' && nodeMap.get('proportion') == null) {
          nodeMap.set('proportion', 1);
        }
      }

      let newDocJson;

      if (isAlreadyMigrated(doc)) {
        const fixedSnapshot = new Uint8Array(doc.export({ mode: 'snapshot' }));
        const currentJson = (await wasm.snapshotToJson(fixedSnapshot)) as Record<string, unknown>;
        const nodes = currentJson.nodes as Record<string, Record<string, unknown>>;
        const reasons: string[] = [];

        const rootNode = nodes['00000000000000000000000000000000'];
        if (rootNode && !rootNode.cascade_attrs) {
          rootNode.cascade_attrs = {
            'style:font_family': 'Pretendard',
            'style:font_size': 1200,
            'style:font_weight': 400,
            'style:text_color': 'black',
            'style:background_color': 'none',
            'style:letter_spacing': 0,
            'paragraph:line_height': 160,
          };
          reasons.push('missing cascade_attrs on root node');
        }

        if ('styles' in currentJson) {
          delete currentJson.styles;
          reasons.push('legacy styles key in document JSON');
        }

        for (const node of Object.values(nodes)) {
          if (node.type !== 'text' || !Array.isArray(node.text)) continue;
          const parentType = nodes[node.parent as string]?.type;
          if (parentType === 'fold_title') {
            for (const seg of node.text as NewTextSegment[]) {
              if (seg.styles && seg.styles.length > 0) {
                seg.styles = [];
                if (!reasons.includes('fold_title text has styles')) {
                  reasons.push('fold_title text has styles');
                }
              }
            }
            continue;
          }
          for (const seg of node.text as NewTextSegment[]) {
            const original = seg.styles ?? [];
            const filled = fillDefaultStyles(original);
            const validated = await validateFontStyles(filled, userId);
            if (filled.length !== original.length || validated !== filled) {
              if (filled.length !== original.length && !reasons.includes('missing default styles')) {
                reasons.push(`missing default styles (had ${original.length}, filled to ${filled.length})`);
              }
              if (validated !== filled && !reasons.includes('font validation changed styles')) {
                reasons.push('font validation changed styles');
              }
              seg.styles = validated;
            }
          }
        }

        if (migrateStyleValues(nodes)) {
          reasons.push('style values need scaling (font_size < 500 or letter_spacing < 5)');
        }

        if (migrateNodeAttrs(nodes)) {
          reasons.push('node attrs need scaling (line_height < 10)');
        }

        if ('settings' in currentJson && migrateSettings(currentJson.settings as Record<string, unknown>)) {
          reasons.push('settings need scaling (block_gap or paragraph_indent < 10)');
        }

        if (fixTextNewlines(nodes)) {
          reasons.push('text segments contain newlines');
        }

        const tableMigrationResult = migrateTableColWidths(nodes);
        if (tableMigrationResult.changed) {
          reasons.push(`table col_width px→ratio (${tableMigrationResult.migratedTableCount} tables)`);
        }
        migratedTables += tableMigrationResult.migratedTableCount;
        skippedMixedTables += tableMigrationResult.skippedMixedTableCount;

        if (reasons.length === 0 && !force) {
          skipped++;
          return;
        }

        if (force && reasons.length === 0) {
          reasons.push('force mode');
        }

        console.log(`[${documentId}] already migrated, re-migrating: ${reasons.join('; ')}`);

        newDocJson = currentJson;
      } else {
        console.log(`[${documentId}] not yet migrated, performing full migration`);
        const settings = doc.getMap('settings').toJSON() as Record<string, unknown>;
        const nodesMap = doc.getMap('nodes');

        const transformedNodes: Record<string, unknown> = {};

        for (const nodeId of nodesMap.keys()) {
          const nodeMap = nodesMap.get(nodeId) as LoroMap;
          const nodeType = nodeMap.get('type') as string;

          if (nodeType === 'text') {
            const textValue = nodeMap.get('text');
            const nodeJson = nodeMap.toJSON() as Record<string, unknown>;
            const parentId = nodeJson.parent as string | undefined;
            const parentType = parentId ? ((nodesMap.get(parentId) as LoroMap | undefined)?.get('type') as string | undefined) : undefined;
            const allowsStyles = parentType !== 'fold_title';

            let newSegments: NewTextSegment[];

            if (textValue instanceof LoroText) {
              const delta = textValue.toDelta() as Delta[];
              newSegments = convertDeltaToSegments(delta, allowsStyles);
            } else {
              const plainText = typeof nodeJson.text === 'string' ? nodeJson.text : '';
              newSegments = plainText ? [{ text: plainText, styles: allowsStyles ? [...DEFAULT_STYLES] : [] }] : [];
            }

            for (const seg of newSegments) {
              if (seg.styles && seg.styles.length > 0) {
                seg.styles = await validateFontStyles(seg.styles, userId);
              }
            }

            transformedNodes[nodeId] = {
              type: 'text',
              text: newSegments,
              children: nodeJson.children ?? [],
              ...(nodeJson.parent == null ? {} : { parent: nodeJson.parent }),
            };
          } else {
            const nodeJson = nodeMap.toJSON() as Record<string, unknown>;
            transformedNodes[nodeId] = normalizeNode(nodeJson);
          }
        }

        fixTextNewlines(transformedNodes as Record<string, Record<string, unknown>>);
        migrateStyleValues(transformedNodes as Record<string, Record<string, unknown>>);
        migrateNodeAttrs(transformedNodes as Record<string, Record<string, unknown>>);
        migrateSettings(settings);
        const tableMigrationResult = migrateTableColWidths(transformedNodes as Record<string, Record<string, unknown>>);
        migratedTables += tableMigrationResult.migratedTableCount;
        skippedMixedTables += tableMigrationResult.skippedMixedTableCount;

        const rootNode = transformedNodes['00000000000000000000000000000000'] as Record<string, unknown> | undefined;
        if (rootNode) {
          rootNode.cascade_attrs = {
            'style:font_family': 'Pretendard',
            'style:font_size': 1200,
            'style:font_weight': 400,
            'style:text_color': 'black',
            'style:background_color': 'none',
            'style:letter_spacing': 0,
            'paragraph:line_height': 160,
          };
        }

        newDocJson = {
          settings,
          nodes: transformedNodes,
        };
      }

      const newSnapshot = await wasm.jsonToSnapshot(newDocJson);
      const newJson = await wasm.snapshotToJson(newSnapshot);

      const newDoc = new LoroDoc();
      newDoc.import(newSnapshot);
      const newVersion = newDoc.version().encode();

      const [updated] = await db
        .update(DocumentContents)
        .set({
          snapshot: newSnapshot,
          json: newJson,
          version: newVersion,
          generation: sql`${DocumentContents.generation} + 1`,
        })
        .where(eq(DocumentContents.id, id))
        .returning({ generation: DocumentContents.generation });

      pubsub.publish('document:sync', documentId, {
        target: '*',
        type: DocumentSyncType.RESET,
        data: JSON.stringify({
          snapshot: newSnapshot.toBase64(),
          version: Buffer.from(newVersion).toString('base64'),
          generation: updated.generation,
        }),
      });

      migrated++;
      if (migrated % 100 === 0) {
        console.log(`Migrated ${migrated} documents...`);
      }
    } catch (err) {
      console.error(`Error migrating document ${documentId}:`, err);
      errors++;
    }
  }

  const pool = new Set<Promise<void>>();
  for (const item of ids) {
    const promise: Promise<void> = migrateDocument(item).then(() => {
      pool.delete(promise);
    });
    pool.add(promise);
    if (pool.size >= CONCURRENCY) {
      await Promise.race(pool);
    }
  }
  await Promise.all(pool);

  console.log(`Migration complete. Migrated: ${migrated}, Skipped: ${skipped}, Errors: ${errors}`);
  console.log(`Table col_width migration: migrated tables=${migratedTables}, skipped mixed tables=${skippedMixedTables}`);

  await pg.end();
  process.exit(0);
})();
