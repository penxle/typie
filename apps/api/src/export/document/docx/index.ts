import {
  AlignmentType,
  convertInchesToTwip, // cspell:disable-line
  Document,
  ExternalHyperlink,
  LevelFormat,
  LineRuleType,
  Packer,
  Paragraph,
  ShadingType,
  Table,
  TableCell,
  TableRow,
  TextRun,
  WidthType,
} from 'docx';
import { inArray } from 'drizzle-orm';
import { db, Embeds } from '@/db';
import { wasm } from '@/utils/wasm';
import {
  convertBlockquote,
  convertCallout,
  convertHardBreak,
  convertHorizontalRule,
  convertListItemParagraph,
  convertPageBreak,
  convertParagraph,
  convertPlaceholderTable,
  NO_BORDER,
  SUBTLE_BORDER,
  toBlockChildren,
} from './blocks';
import { convertImage, loadImages } from './image';
import { createRubyRun } from './ruby';
import { convertTable, convertTableCell, convertTableRow } from './table';
import { convertTextSegments } from './text';
import type { ImageAsset } from '../external';
import type { BlockquoteParagraph, DocDefaults } from './blocks';
import type { TextConvertContext, TextSegment } from './text';

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export type FileChild = Paragraph | Table | any;

type NodeEntry = Record<string, unknown> & {
  type: string;
  children?: string[];
  parent?: string;
};

type DocumentJson = {
  settings: Record<string, unknown>;
  nodes: Record<string, NodeEntry>;
};

export type ConvertContext = {
  nodes: Record<string, NodeEntry>;
  assets: Map<string, ImageAsset>;
  embeds: Map<string, { url: string; title: string | null }>;
  textCtx: TextConvertContext;
  numberingRef: string;
  listStack: { type: 'bullet' | 'ordered'; depth: number }[];
  paragraphIndentTwips: number;
  docDefaults: DocDefaults;
  contentWidthPx: number;
};

export type GenerateDocumentDocxParams = {
  snapshot: Uint8Array;
  title: string;
  author: string;
  pageWidth: number;
  pageHeight: number;
  pageMarginTop: number;
  pageMarginBottom: number;
  pageMarginLeft: number;
  pageMarginRight: number;
};

export async function generateDocumentDocx(params: GenerateDocumentDocxParams): Promise<Uint8Array> {
  const { snapshot, title, author, pageWidth, pageHeight, pageMarginTop, pageMarginBottom, pageMarginLeft, pageMarginRight } = params;

  // 1. snapshotToJson으로 시맨틱 데이터 추출
  const json = (await wasm.snapshotToJson(snapshot)) as unknown as DocumentJson;

  // 2. 이미지 노드 수집 & S3에서 로딩, 임베드 ID 수집 & DB 조회
  const imageIds = collectNodeIds(json.nodes, 'image');
  const embedIds = collectNodeIds(json.nodes, 'embed');

  const [assets, embeds] = await Promise.all([loadImages(imageIds), loadEmbeds(embedIds)]);

  // 3. root 노드 찾기
  const rootId = Object.keys(json.nodes).find((id) => json.nodes[id].type === 'root');
  if (!rootId) {
    throw new Error('Root node not found in document');
  }

  // cascade_attrs에서 기본 폰트/크기 추출
  const rootEntry = json.nodes[rootId];
  const cascadeAttrs = rootEntry.cascade_attrs as Record<string, unknown> | undefined;
  const defaultFont = (cascadeAttrs?.['style:font_family'] as string) ?? 'Pretendard';
  const defaultFontSizePt100 = (cascadeAttrs?.['style:font_size'] as number) ?? 1200;
  const defaultFontSizeHp = Math.round((defaultFontSizePt100 / 100) * 2);
  // line_height: × 100 (e.g. 160 = 160%). AT_LEAST 모드에서는 twips 절대값 사용 (fontSizePt × ratio × 20)
  const defaultLineHeight = (cascadeAttrs?.['paragraph:line_height'] as number) ?? 160;
  const defaultFontSizePt = defaultFontSizePt100 / 100;
  const defaultLineSpacingTwips = Math.round(defaultFontSizePt * (defaultLineHeight / 100) * 20);

  const numberingRef = 'typie-ordered';

  // paragraph_indent: × 100 스케일, 기본 폰트 16px 기준 → px → twips (1px = 15twips)
  const paragraphIndentRaw = (json.settings.paragraph_indent as number) ?? 100;
  const paragraphIndentPx = (paragraphIndentRaw / 100) * 16;
  const paragraphIndentTwips = Math.round(paragraphIndentPx * 15);

  // block_gap: × 100 스케일, 기본 폰트 16px 기준 → px → twips
  const blockGapRaw = (json.settings.block_gap as number) ?? 100;
  const blockGapPx = (blockGapRaw / 100) * 16;
  const blockGapTwips = Math.round(blockGapPx * 15);

  // CSS px → twips (1 inch = 96px = 1440 twips, 1px = 15 twips)
  const PX_TO_TWIPS = 15;
  const pageWidthTwips = Math.round(pageWidth * PX_TO_TWIPS);
  const pageHeightTwips = Math.round(pageHeight * PX_TO_TWIPS);
  const contentWidthPx = pageWidth - pageMarginLeft - pageMarginRight;

  const docDefaults: DocDefaults = { fontSizePt: defaultFontSizePt, blockGapTwips };
  const ctx: ConvertContext = {
    nodes: json.nodes,
    assets,
    embeds,
    textCtx: { createRubyRun },
    numberingRef,
    listStack: [],
    paragraphIndentTwips,
    docDefaults,
    contentWidthPx,
  };

  // 4. root의 children을 순회하며 변환 + Table 뒤에 block_gap spacer 삽입
  const sections = withTableSpacers(
    (rootEntry.children ?? []).flatMap((childId) => convertNode(childId, ctx, true)),
    blockGapTwips,
  );

  // 5. Document 생성
  const doc = new Document({
    title,
    creator: author,
    description: '타이피(https://typie.co)에서 만든 문서',
    styles: {
      default: {
        document: {
          run: {
            font: defaultFont,
            size: defaultFontSizeHp,
          },
          paragraph: {
            spacing: { after: blockGapTwips, line: defaultLineSpacingTwips, lineRule: LineRuleType.AT_LEAST },
          },
        },
      },
    },
    numbering: {
      config: [
        {
          reference: numberingRef,
          levels: Array.from({ length: 9 }, (_, i) => ({
            level: i,
            format: LevelFormat.DECIMAL,
            text: `%${i + 1}.`,
            alignment: AlignmentType.START,
            style: {
              paragraph: {
                indent: {
                  left: convertInchesToTwip(0.5 * (i + 1)), // cspell:disable-line
                  hanging: convertInchesToTwip(0.25), // cspell:disable-line
                },
              },
            },
          })),
        },
      ],
    },
    sections: [
      {
        properties: {
          page: {
            size: { width: pageWidthTwips, height: pageHeightTwips },
            margin: {
              top: Math.round(pageMarginTop * PX_TO_TWIPS),
              bottom: Math.round(pageMarginBottom * PX_TO_TWIPS),
              left: Math.round(pageMarginLeft * PX_TO_TWIPS),
              right: Math.round(pageMarginRight * PX_TO_TWIPS),
            },
          },
        },
        children: sections,
      },
    ],
  });

  // 6. DOCX 바이너리 생성
  const buffer = await Packer.toBuffer(doc);
  return new Uint8Array(buffer);
}

function convertNode(nodeId: string, ctx: ConvertContext, isRootChild = false): FileChild[] {
  const entry = ctx.nodes[nodeId];
  if (!entry) return [];

  switch (entry.type) {
    case 'paragraph': {
      const inlineChildren = collectInlineChildren(entry, ctx);
      const indent = isRootChild ? ctx.paragraphIndentTwips : 0;
      return [convertParagraph(entry as { align?: string; line_height?: number }, inlineChildren, indent, ctx.docDefaults)];
    }

    case 'blockquote': {
      const variant = (entry as { variant?: string }).variant;
      const collectCtx = variant === 'message_sent' ? { ...ctx, textCtx: { ...ctx.textCtx, defaultColor: 'FFFFFF' } } : ctx;
      const paragraphs: BlockquoteParagraph[] = [];
      for (const childId of entry.children ?? []) {
        const childEntry = ctx.nodes[childId];
        if (!childEntry || childEntry.type !== 'paragraph') continue;
        paragraphs.push({
          inlineChildren: collectInlineChildren(childEntry, collectCtx),
          align: childEntry.align as string | undefined,
          lineHeight: childEntry.line_height as number | undefined,
        });
      }
      return convertBlockquote(entry as { variant?: string }, paragraphs, ctx.docDefaults);
    }

    case 'callout': {
      const innerElements = convertChildren(entry, ctx);
      return [convertCallout(entry as { variant?: string }, innerElements)];
    }

    case 'horizontal_rule': {
      return [convertHorizontalRule()];
    }

    case 'page_break': {
      return [convertPageBreak()];
    }

    case 'bullet_list': {
      ctx.listStack.push({ type: 'bullet', depth: ctx.listStack.length });
      const items = convertChildren(entry, ctx);
      ctx.listStack.pop();
      return items;
    }

    case 'ordered_list': {
      ctx.listStack.push({ type: 'ordered', depth: ctx.listStack.length });
      const items = convertChildren(entry, ctx);
      ctx.listStack.pop();
      return items;
    }

    case 'list_item': {
      const currentList = ctx.listStack.at(-1);
      const listType = currentList?.type ?? 'bullet';
      const level = currentList?.depth ?? 0;

      const results: FileChild[] = [];
      for (const childId of entry.children ?? []) {
        const childEntry = ctx.nodes[childId];
        if (!childEntry) continue;

        if (childEntry.type === 'paragraph') {
          const inlineChildren = collectInlineChildren(childEntry, ctx);
          results.push(
            convertListItemParagraph(
              inlineChildren,
              listType,
              level,
              ctx.numberingRef,
              childEntry as { align?: string; line_height?: number },
              ctx.docDefaults,
            ),
          );
        } else {
          results.push(...convertNode(childId, ctx));
        }
      }
      return results;
    }

    case 'table': {
      const rows: TableRow[] = [];
      for (const childId of entry.children ?? []) {
        const childEntry = ctx.nodes[childId];
        if (!childEntry || childEntry.type !== 'table_row') continue;

        const cells: TableCell[] = [];
        for (const cellId of childEntry.children ?? []) {
          const cellEntry = ctx.nodes[cellId];
          if (!cellEntry || cellEntry.type !== 'table_cell') continue;

          const cellChildren = convertChildren(cellEntry, ctx);
          cells.push(convertTableCell(cellEntry as { col_width?: number | null }, cellChildren));
        }

        rows.push(convertTableRow(cells));
      }

      return [convertTable(entry as { border_style?: string; align?: string; proportion?: number }, rows)];
    }

    case 'fold': {
      return [convertFold(entry, ctx)];
    }

    case 'image': {
      return [convertImage(entry as unknown as { type: 'image'; id?: string; proportion: number }, ctx.assets, ctx.contentWidthPx)];
    }

    case 'embed': {
      return [convertEmbed(entry, ctx.embeds)];
    }

    case 'file':
    case 'archived': {
      const label = entry.type === 'file' ? '파일' : '보관된 블록';
      return [convertPlaceholderTable(`[${label}]`)];
    }

    case 'text':
    case 'hard_break':
    case 'root': {
      return [];
    }

    default: {
      return [];
    }
  }
}

function withTableSpacers(elements: FileChild[], blockGapTwips: number): FileChild[] {
  const result: FileChild[] = [];
  for (let i = 0; i < elements.length; i++) {
    result.push(elements[i]);
    if (elements[i] instanceof Table && i < elements.length - 1) {
      result.push(new Paragraph({ spacing: { before: blockGapTwips, after: 0, line: 0 }, children: [] }));
    }
  }
  return result;
}

function convertChildren(entry: NodeEntry, ctx: ConvertContext): FileChild[] {
  const results: FileChild[] = [];
  for (const childId of entry.children ?? []) {
    results.push(...convertNode(childId, ctx));
  }
  return results;
}

function collectInlineChildren(entry: NodeEntry, ctx: ConvertContext): (TextRun | ReturnType<typeof convertTextSegments>[number])[] {
  const result: (TextRun | ReturnType<typeof convertTextSegments>[number])[] = [];

  for (const childId of entry.children ?? []) {
    const childEntry = ctx.nodes[childId];
    if (!childEntry) continue;

    if (childEntry.type === 'text') {
      const segments = ((childEntry.text as TextSegment[]) ?? []) as TextSegment[];
      result.push(...convertTextSegments(segments, ctx.textCtx));
    } else if (childEntry.type === 'hard_break') {
      result.push(convertHardBreak());
    }
  }

  return result;
}

function convertFold(entry: NodeEntry, ctx: ConvertContext): Table {
  let titleChildren: (TextRun | ReturnType<typeof convertTextSegments>[number])[] = [];
  const contentElements: FileChild[] = [];

  for (const childId of entry.children ?? []) {
    const childEntry = ctx.nodes[childId];
    if (!childEntry) continue;

    if (childEntry.type === 'fold_title') {
      titleChildren = collectInlineChildren(childEntry, ctx);
    } else if (childEntry.type === 'fold_content') {
      contentElements.push(...convertChildren(childEntry, ctx));
    }
  }

  const titleParagraph = new Paragraph({
    children: [new TextRun({ text: '\u25B6 ', bold: true }), ...titleChildren],
    run: { bold: true },
    spacing: { after: 0 },
  });

  return new Table({
    rows: [
      new TableRow({
        children: [
          new TableCell({
            children: [titleParagraph],
            shading: { fill: 'F3F4F9', type: ShadingType.CLEAR },
            borders: { top: SUBTLE_BORDER, bottom: SUBTLE_BORDER, left: SUBTLE_BORDER, right: SUBTLE_BORDER },
            margins: { top: 80, bottom: 80, left: 120, right: 120 },
          }),
        ],
      }),
      new TableRow({
        children: [
          new TableCell({
            children: toBlockChildren(contentElements),
            borders: { top: NO_BORDER, bottom: SUBTLE_BORDER, left: SUBTLE_BORDER, right: SUBTLE_BORDER },
            margins: { top: 80, bottom: 80, left: 120, right: 120 },
          }),
        ],
      }),
    ],
    width: { size: 100, type: WidthType.PERCENTAGE },
  });
}

function collectNodeIds(nodes: Record<string, { type: string; id?: string }>, type: string): string[] {
  const ids: string[] = [];
  for (const entry of Object.values(nodes)) {
    if (entry.type === type && entry.id) {
      ids.push(entry.id);
    }
  }
  return ids;
}

function convertEmbed(entry: NodeEntry, embeds: Map<string, { url: string; title: string | null }>): Table {
  const embedId = entry.id as string | undefined;
  const embedData = embedId ? embeds.get(embedId) : undefined;

  if (!embedData) {
    return convertPlaceholderTable('[임베드]');
  }

  const label = embedData.title || embedData.url;
  return new Table({
    alignment: AlignmentType.CENTER,
    width: { size: 50, type: WidthType.PERCENTAGE },
    rows: [
      new TableRow({
        children: [
          new TableCell({
            children: [
              new Paragraph({
                alignment: AlignmentType.CENTER,
                spacing: { after: 0 },
                children: [
                  new ExternalHyperlink({
                    children: [new TextRun({ text: label, style: 'Hyperlink' })],
                    link: embedData.url,
                  }),
                ],
              }),
            ],
            borders: { top: SUBTLE_BORDER, bottom: SUBTLE_BORDER, left: SUBTLE_BORDER, right: SUBTLE_BORDER },
            margins: { top: 100, bottom: 100, left: 160, right: 160 },
          }),
        ],
      }),
    ],
  });
}

async function loadEmbeds(ids: string[]): Promise<Map<string, { url: string; title: string | null }>> {
  if (ids.length === 0) return new Map();
  const rows = await db.select({ id: Embeds.id, url: Embeds.url, title: Embeds.title }).from(Embeds).where(inArray(Embeds.id, ids));
  return new Map(rows.map((r) => [r.id, { url: r.url, title: r.title }]));
}
