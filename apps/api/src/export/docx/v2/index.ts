import {
  AlignmentType,
  convertInchesToTwip, // cspell:disable-line
  Document,
  ExternalHyperlink,
  ImageRun,
  LevelFormat,
  LineRuleType,
  Packer,
  Paragraph,
  Table,
  TableCell,
  TableRow,
  TextRun,
  WidthType,
} from 'docx';
import { mapFormat } from '../../core/assets.ts';
import { findFontFamily, nearestWeight } from '../../core/fonts.ts';
import { parseDocumentV2 } from '../../core/v2/document.ts';
import { traverseV2 } from '../../core/v2/traverse.ts';
import { SUBTLE_BORDER } from '../blocks.ts';
import {
  buildBlockquoteV2,
  buildCalloutV2,
  buildFoldV2,
  buildHorizontalRuleV2,
  buildListItemParagraphV2,
  buildParagraphV2,
  buildPlaceholderTableV2,
} from './blocks.ts';
import { convertTableV2 } from './table.ts';
import { buildRunOptionsV2, runsToComponents } from './text.ts';
import type { ExportFontFamily, PageLayout } from '../../core/types.ts';
import type { NodeVisitorV2 } from '../../core/v2/types.ts';
import type { DocDefaults } from '../blocks.ts';
import type { FileChild } from '../index.ts';
import type { TextConvertContextV2 } from './text.ts';

export type ConvertContextV2 = {
  textCtx: TextConvertContextV2;
  numberingRef: string;
  listStack: { type: 'bullet' | 'ordered'; depth: number }[];
  paragraphIndentTwips: number;
  docDefaults: DocDefaults;
  contentWidthPx: number;
};

export type GenerateDocumentDocxV2Params = {
  graph: Uint8Array;
  title: string;
  author: string;
  fonts: ExportFontFamily[];
  layout: PageLayout;
};

const docxVisitorV2: NodeVisitorV2<ConvertContextV2, FileChild[]> = {
  paragraph: (p, ctx) => {
    const inlineChildren = runsToComponents(p, ctx.textCtx);
    const listCtx = ctx.listStack.at(-1);

    if (listCtx) {
      return [
        buildListItemParagraphV2(p.align, p.lineHeight, inlineChildren, listCtx.type, listCtx.depth, ctx.numberingRef, ctx.docDefaults),
      ];
    }

    return [buildParagraphV2(p.align, p.lineHeight, inlineChildren, ctx.paragraphIndentTwips, ctx.docDefaults)];
  },

  table: (t) => [convertTableV2(t)],

  image: (n, ctx) => {
    const asset = n.asset;
    if (asset.width <= 0 || asset.height <= 0) {
      return [buildPlaceholderTableV2('[이미지를 불러올 수 없습니다]')];
    }

    const displayWidth = ctx.contentWidthPx * Math.min(n.proportion, 1);
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

  file: () => [buildPlaceholderTableV2('[파일]')],

  embed: (n) => {
    if (!n.data) {
      return [buildPlaceholderTableV2('[임베드]')];
    }

    const label = n.data.title || n.data.url;
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
                        link: n.data.url,
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

  archived: () => [buildPlaceholderTableV2('[보관된 블록]')],

  horizontalRule: () => [buildHorizontalRuleV2()],

  // eslint-disable-next-line unicorn/no-magic-array-flat-depth
  bulletList: (items) => items.flat(2),
  // eslint-disable-next-line unicorn/no-magic-array-flat-depth
  orderedList: (items) => items.flat(2),

  blockquote: (variant, children) => buildBlockquoteV2(variant, children),

  callout: (variant, children) => [buildCalloutV2(variant, children)],

  fold: (title, content, ctx) => {
    const titleChildren = title.map((run) => new TextRun({ ...buildRunOptionsV2(run.style, ctx.textCtx), text: run.text }));
    return [buildFoldV2(titleChildren, content)];
  },

  onEnterList: (type, depth, ctx) => {
    ctx.listStack.push({ type, depth });
  },

  onExitList: (ctx) => {
    ctx.listStack.pop();
  },
};

export async function generateDocumentDocxV2(params: GenerateDocumentDocxV2Params): Promise<Uint8Array> {
  const { title, author, layout } = params;
  const { pageWidth, pageHeight, pageMarginTop, pageMarginBottom, pageMarginLeft, pageMarginRight } = layout;

  const parsed = await parseDocumentV2(params.graph);
  const { defaults } = parsed;

  const defaultFam = findFontFamily(params.fonts, defaults.fontFamily);
  const defaultFontEntry = defaultFam ? nearestWeight(defaultFam.weights, 400) : undefined;
  const resolvedDefaultFont = defaultFontEntry?.postScriptName ?? defaults.fontFamily;
  const defaultFontSizeHp = Math.round((defaults.fontSizePt100 / 100) * 2);
  const defaultFontSizePt = defaults.fontSizePt100 / 100;
  const defaultLineSpacingTwips = Math.round(defaultFontSizePt * (defaults.lineHeight / 100) * 20);

  const numberingRef = 'typie-ordered';

  const PX_TO_TWIPS = 15;
  const paragraphIndentTwips = Math.round(defaults.paragraphIndentPx * PX_TO_TWIPS);
  const blockGapTwips = Math.round(defaults.blockGapPx * PX_TO_TWIPS);
  const pageWidthTwips = Math.round(pageWidth * PX_TO_TWIPS);
  const pageHeightTwips = Math.round(pageHeight * PX_TO_TWIPS);
  const contentWidthPx = pageWidth - pageMarginLeft - pageMarginRight;

  const docDefaults: DocDefaults = { fontSizePt: defaultFontSizePt, blockGapTwips };
  const ctx: ConvertContextV2 = {
    textCtx: { fonts: params.fonts, defaultFamilyName: defaults.fontFamily },
    numberingRef,
    listStack: [],
    paragraphIndentTwips,
    docDefaults,
    contentWidthPx,
  };

  const bodyChunks = traverseV2(parsed, docxVisitorV2, ctx);
  const sections = withTableSpacers(bodyChunks.flat(), blockGapTwips);

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
