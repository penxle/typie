#!/usr/bin/env tsx

import { eq, sql } from 'drizzle-orm';
import { LoroDoc, LoroMap, LoroText } from 'loro-crdt';
import { db, DocumentContents, pg } from '@/db';
import { DocumentSyncType } from '@/enums';
import { pubsub } from '@/pubsub';
import { jsonToSnapshot, snapshotToJson } from '@/utils/wasm';

process.env.SCRIPT = 'true';

const force = process.argv.includes('--force');

type Delta = {
  insert: string;
  attributes?: Record<string, unknown>;
};

type NewStyle =
  | { type: 'font_weight'; weight: number }
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
  { type: 'font_size', size: 12 },
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

await (async () => {
  if (force) {
    console.log('Force mode enabled — re-migrating all documents');
  }

  console.log('Starting mark → style/annotation migration...');

  const ids = await db
    .select({
      id: DocumentContents.id,
      documentId: DocumentContents.documentId,
    })
    .from(DocumentContents);

  console.log(`Found ${ids.length} documents to process`);

  let migrated = 0;
  let skipped = 0;
  let errors = 0;
  let migratedTables = 0;
  let skippedMixedTables = 0;

  for (const { id, documentId } of ids) {
    try {
      const [row] = await db.select({ snapshot: DocumentContents.snapshot }).from(DocumentContents).where(eq(DocumentContents.id, id));

      if (!row) {
        skipped++;
        continue;
      }

      const doc = new LoroDoc();
      doc.import(row.snapshot);

      // Fix table nodes missing proportion field (added in a later migration)
      const allNodesMap = doc.getMap('nodes');
      for (const nodeId of allNodesMap.keys()) {
        const nodeMap = allNodesMap.get(nodeId) as LoroMap;
        if ((nodeMap.get('type') as string) === 'table' && nodeMap.get('proportion') == null) {
          nodeMap.set('proportion', 1);
        }
      }

      let newDocJson;

      if (isAlreadyMigrated(doc)) {
        const fixedSnapshot = new Uint8Array(doc.export({ mode: 'snapshot' }));
        const currentJson = (await snapshotToJson(fixedSnapshot)) as Record<string, unknown>;
        const nodes = currentJson.nodes as Record<string, Record<string, unknown>>;
        let needsFix = false;

        const rootNode = nodes['00000000000000000000000000000000'];
        if (rootNode && !rootNode.cascade_attrs) {
          rootNode.cascade_attrs = {
            'style:font_family': 'Pretendard',
            'style:font_size': 12,
            'style:font_weight': 400,
            'style:text_color': 'black',
            'style:background_color': 'none',
            'style:letter_spacing': 0,
            'paragraph:line_height': 1.6,
          };
          needsFix = true;
        }

        if ('styles' in currentJson) {
          delete currentJson.styles;
          needsFix = true;
        }

        for (const node of Object.values(nodes)) {
          if (node.type !== 'text' || !Array.isArray(node.text)) continue;
          const parentType = nodes[node.parent as string]?.type;
          if (parentType === 'fold_title') {
            for (const seg of node.text as NewTextSegment[]) {
              if (seg.styles && seg.styles.length > 0) {
                seg.styles = [];
                needsFix = true;
              }
            }
            continue;
          }
          for (const seg of node.text as NewTextSegment[]) {
            const filled = fillDefaultStyles(seg.styles ?? []);
            if (filled.length !== (seg.styles ?? []).length) {
              seg.styles = filled;
              needsFix = true;
            }
          }
        }

        if (fixTextNewlines(nodes)) {
          needsFix = true;
        }

        const tableMigrationResult = migrateTableColWidths(nodes);
        if (tableMigrationResult.changed) {
          needsFix = true;
        }
        migratedTables += tableMigrationResult.migratedTableCount;
        skippedMixedTables += tableMigrationResult.skippedMixedTableCount;

        if (!needsFix && !force) {
          skipped++;
          continue;
        }

        newDocJson = currentJson;
      } else {
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
        const tableMigrationResult = migrateTableColWidths(transformedNodes as Record<string, Record<string, unknown>>);
        migratedTables += tableMigrationResult.migratedTableCount;
        skippedMixedTables += tableMigrationResult.skippedMixedTableCount;

        const rootNode = transformedNodes['00000000000000000000000000000000'] as Record<string, unknown> | undefined;
        if (rootNode) {
          rootNode.cascade_attrs = {
            'style:font_family': 'Pretendard',
            'style:font_size': 12,
            'style:font_weight': 400,
            'style:text_color': 'black',
            'style:background_color': 'none',
            'style:letter_spacing': 0,
            'paragraph:line_height': 1.6,
          };
        }

        newDocJson = {
          settings,
          nodes: transformedNodes,
        };
      }

      const newSnapshot = await jsonToSnapshot(newDocJson);
      const newJson = await snapshotToJson(newSnapshot);

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

  console.log(`Migration complete. Migrated: ${migrated}, Skipped: ${skipped}, Errors: ${errors}`);
  console.log(`Table col_width migration: migrated tables=${migratedTables}, skipped mixed tables=${skippedMixedTables}`);

  await pg.end();
  process.exit(0);
})();
