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
  return stylesMap.size > 0;
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

  for (const { id, documentId } of ids) {
    try {
      const [row] = await db.select({ snapshot: DocumentContents.snapshot }).from(DocumentContents).where(eq(DocumentContents.id, id));

      if (!row) {
        skipped++;
        continue;
      }

      const doc = new LoroDoc();
      doc.import(row.snapshot);

      let newDocJson;

      if (isAlreadyMigrated(doc)) {
        const currentJson = (await snapshotToJson(row.snapshot)) as Record<string, unknown>;
        const styles = currentJson.styles as Record<string, unknown>;
        const nodes = currentJson.nodes as Record<string, Record<string, unknown>>;
        let needsFix = false;

        if (!('line_height' in styles)) {
          styles.line_height = 1.6;
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

        newDocJson = {
          settings,
          styles: {
            font_family: 'Pretendard',
            font_size: 12,
            font_weight: 400,
            text_color: 'black',
            background_color: 'none',
            letter_spacing: 0,
            line_height: 1.6,
            italic: false,
            strikethrough: false,
            underline: false,
          },
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

  await pg.end();
  process.exit(0);
})();
