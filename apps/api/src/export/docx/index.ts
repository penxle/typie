// cspell:ignore Segs
import {
  AlignmentType,
  convertInchesToTwip, // cspell:disable-line
  Document,
  ExternalHyperlink,
  ImageRun,
  LevelFormat,
  LineRuleType,
  Packer,
  PageBreak,
  Paragraph,
  ShadingType,
  Table,
  TableCell,
  TableRow,
  TextRun,
  WidthType,
} from 'docx';
import { mapFormat } from '../core/assets';
import { parseDocument } from '../core/document';
import { findFontFamily, nearestWeight } from '../core/fonts';
import { traverse } from '../core/traverse';
import {
  convertBlockquote,
  convertCallout,
  convertHardBreak,
  convertHorizontalRule,
  convertListItemParagraph,
  convertParagraph,
  convertPlaceholderTable,
  NO_BORDER,
  SUBTLE_BORDER,
  toBlockChildren,
} from './blocks';
import { createRubyRun } from './ruby';
import { convertTable, convertTableCell, convertTableRow } from './table';
import { convertTextSegments } from './text';
import type { XmlComponent } from 'docx';
import type { NodeVisitor } from '../core/traverse';
import type { Annotation, EmbedInfo, ExportFontFamily, ImageAsset, InlineSegment, NodeEntry, Style, TextSegment } from '../core/types';
import type { BlockquoteParagraph, DocDefaults } from './blocks';
import type { TextConvertContext } from './text';

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export type FileChild = Paragraph | Table | any;

export type ConvertContext = {
  nodes: Record<string, NodeEntry>;
  assets: Map<string, ImageAsset>;
  embeds: Map<string, EmbedInfo>;
  textCtx: TextConvertContext;
  numberingRef: string;
  listStack: { type: 'bullet' | 'ordered'; depth: number }[];
  paragraphIndentTwips: number;
  docDefaults: DocDefaults;
  contentWidthPx: number;
  nestingDepth: number;
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
  fonts: ExportFontFamily[];
};

function convertInlineSegments(segments: InlineSegment[], textCtx: TextConvertContext) {
  const result: (TextRun | ExternalHyperlink | XmlComponent | PageBreak)[] = [];
  for (const seg of segments) {
    switch (seg.type) {
      case 'text': {
        result.push(...convertTextSegments([seg], textCtx));
        break;
      }
      case 'hard_break': {
        result.push(convertHardBreak());
        break;
      }
      case 'page_break': {
        result.push(new PageBreak());
        break;
      }
    }
  }
  return result;
}

function collectInlineChildren(entry: NodeEntry, ctx: ConvertContext) {
  const result: (TextRun | ExternalHyperlink | XmlComponent)[] = [];
  for (const childId of entry.children ?? []) {
    const childEntry = ctx.nodes[childId];
    if (!childEntry) continue;
    if (childEntry.type === 'text') {
      const rawSegs = (childEntry.text ?? []) as { text: string; styles: Style[]; annotations: Annotation[] }[];
      result.push(
        ...convertTextSegments(
          rawSegs.map((s): TextSegment => ({ type: 'text', ...s })),
          ctx.textCtx,
        ),
      );
    } else if (childEntry.type === 'hard_break') {
      result.push(convertHardBreak());
    }
  }
  return result;
}

const docxVisitor: NodeVisitor<ConvertContext, FileChild[]> = {
  paragraph: (node, ctx) => {
    const inlineChildren = convertInlineSegments(node.segments, ctx.textCtx);
    const listCtx = ctx.listStack.at(-1);

    if (listCtx) {
      return [
        convertListItemParagraph(
          inlineChildren,
          listCtx.type,
          listCtx.depth,
          ctx.numberingRef,
          node.attrs as { align?: string; line_height?: number },
          ctx.docDefaults,
        ),
      ];
    }

    const indent = ctx.nestingDepth === 0 ? ctx.paragraphIndentTwips : 0;
    return [convertParagraph(node.attrs as { align?: string; line_height?: number }, inlineChildren, indent, ctx.docDefaults)];
  },

  table: (entry, convertChildren, ctx) => {
    ctx.nestingDepth++;
    const rows: TableRow[] = [];
    for (const childId of entry.children ?? []) {
      const childEntry = ctx.nodes[childId];
      if (!childEntry || childEntry.type !== 'table_row') continue;

      const cells: TableCell[] = [];
      for (const cellId of childEntry.children ?? []) {
        const cellEntry = ctx.nodes[cellId];
        if (!cellEntry || cellEntry.type !== 'table_cell') continue;

        const cellChildren = convertChildren(cellEntry).flat();
        cells.push(convertTableCell(cellEntry as { col_width?: number | null }, cellChildren));
      }

      rows.push(convertTableRow(cells));
    }
    ctx.nestingDepth--;
    return [convertTable(entry as { border_style?: string; align?: string; proportion?: number }, rows)];
  },

  image: (node, asset, ctx) => {
    if (asset.width <= 0 || asset.height <= 0) {
      return [convertPlaceholderTable('[이미지를 불러올 수 없습니다]')];
    }

    const proportion = (node.attrs.proportion as number) ?? 1;
    const displayWidth = ctx.contentWidthPx * Math.min(proportion, 1);
    const displayHeight = displayWidth * (asset.height / asset.width);

    return [
      new Paragraph({
        alignment: AlignmentType.CENTER,
        children: [
          new ImageRun({
            type: mapFormat(asset.format) as 'jpg' | 'png' | 'gif' | 'bmp',
            data: asset.bytes,
            transformation: { width: Math.round(displayWidth), height: Math.round(displayHeight) },
          }),
        ],
      }),
    ];
  },

  file: () => [convertPlaceholderTable('[파일]')],

  embed: (_id, data) => {
    if (!data) {
      return [convertPlaceholderTable('[임베드]')];
    }

    const label = data.title || data.url;
    return [
      new Table({
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
                        link: data.url,
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
      }),
    ];
  },

  archived: () => [convertPlaceholderTable('[보관된 블록]')],

  horizontalRule: () => [convertHorizontalRule()],

  // eslint-disable-next-line unicorn/no-magic-array-flat-depth
  bulletList: (items) => items.flat(2),
  // eslint-disable-next-line unicorn/no-magic-array-flat-depth
  orderedList: (items) => items.flat(2),

  blockquote: (entry, variant, _convertChildren, ctx) => {
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
  },

  callout: (entry, _variant, convertChildren, ctx) => {
    ctx.nestingDepth++;
    const innerElements = convertChildren(entry).flat();
    ctx.nestingDepth--;
    return [convertCallout(entry as { variant?: string }, innerElements)];
  },

  fold: (entry, convertChildren, ctx) => {
    let titleChildren: ReturnType<typeof collectInlineChildren> = [];
    const contentElements: FileChild[] = [];

    for (const childId of entry.children ?? []) {
      const childEntry = ctx.nodes[childId];
      if (!childEntry) continue;

      if (childEntry.type === 'fold_title') {
        titleChildren = collectInlineChildren(childEntry, ctx);
      } else if (childEntry.type === 'fold_content') {
        ctx.nestingDepth++;
        contentElements.push(...convertChildren(childEntry).flat());
        ctx.nestingDepth--;
      }
    }

    const titleParagraph = new Paragraph({
      children: [new TextRun({ text: '\u25B6 ', bold: true }), ...(titleChildren ?? [])],
      run: { bold: true },
      spacing: { after: 0 },
    });

    return [
      new Table({
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
      }),
    ];
  },

  onEnterList: (type, depth, ctx) => {
    ctx.listStack.push({ type, depth });
  },

  onExitList: (ctx) => {
    ctx.listStack.pop();
  },
};

export async function generateDocumentDocx(params: GenerateDocumentDocxParams): Promise<Uint8Array> {
  const { title, author, pageWidth, pageHeight, pageMarginTop, pageMarginBottom, pageMarginLeft, pageMarginRight } = params;

  const parsed = await parseDocument(params.snapshot);
  const { defaults } = parsed;

  const defaultFam = findFontFamily(params.fonts, defaults.fontFamily);
  const defaultFontEntry = defaultFam ? nearestWeight(defaultFam.weights, 400) : undefined;
  const resolvedDefaultFont = defaultFontEntry?.postScriptName ?? defaults.fontFamily;
  const defaultFontSizeHp = Math.round((defaults.fontSizePt100 / 100) * 2);
  const defaultFontSizePt = defaults.fontSizePt100 / 100;
  // line_height: × 100 (e.g. 160 = 160%). AT_LEAST 모드에서는 twips 절대값 사용 (fontSizePt × ratio × 20)
  const defaultLineSpacingTwips = Math.round(defaultFontSizePt * (defaults.lineHeight / 100) * 20);

  const numberingRef = 'typie-ordered';

  // CSS px → twips (1 inch = 96px = 1440 twips, 1px = 15 twips)
  const PX_TO_TWIPS = 15;
  const paragraphIndentTwips = Math.round(defaults.paragraphIndentPx * PX_TO_TWIPS);
  const blockGapTwips = Math.round(defaults.blockGapPx * PX_TO_TWIPS);
  const pageWidthTwips = Math.round(pageWidth * PX_TO_TWIPS);
  const pageHeightTwips = Math.round(pageHeight * PX_TO_TWIPS);
  const contentWidthPx = pageWidth - pageMarginLeft - pageMarginRight;

  const docDefaults: DocDefaults = { fontSizePt: defaultFontSizePt, blockGapTwips };
  const ctx: ConvertContext = {
    nodes: parsed.nodes,
    assets: parsed.images,
    embeds: parsed.embeds,
    textCtx: { createRubyRun, fonts: params.fonts, defaultFamilyName: defaults.fontFamily },
    numberingRef,
    listStack: [],
    paragraphIndentTwips,
    docDefaults,
    contentWidthPx,
    nestingDepth: 0,
  };

  const bodyChunks = traverse(parsed, docxVisitor, ctx);
  const sections = withTableSpacers(bodyChunks.flat(), blockGapTwips);

  // Document 생성
  const doc = new Document({
    title,
    creator: author,
    description: '타이피(https://typie.co)에서 만든 문서',
    styles: {
      default: {
        document: {
          run: {
            font: resolvedDefaultFont,
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

  // DOCX 바이너리 생성
  const buffer = await Packer.toBuffer(doc);
  return new Uint8Array(buffer);
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
