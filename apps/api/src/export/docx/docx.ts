import { defaultValues } from '@typie/ui/tiptap/values-base';
import { AlignmentType, Document, Packer, PageBreak, Paragraph, Table } from 'docx';
import { match } from 'ts-pattern';
import { PostLayoutMode } from '@/enums';
import {
  convertBlockquoteToParagraphs,
  convertCalloutToParagraphs,
  convertCodeBlock,
  convertEmbed,
  convertFile,
  convertFold,
  convertHorizontalRule,
  convertHtmlBlock,
  convertImage,
  convertListToParagraphs,
  convertParagraph,
  convertTable,
  downloadAllImages,
} from './nodes';
import { mmToTwips, pxToHalfPt } from './utils/unit';
import type { JSONContent } from '@tiptap/core';
import type { ConvertOptions } from './types';
import type { FontMapper } from './utils/font-mapping';

export async function generatePostDocx(params: {
  title?: string | null;
  subtitle?: string | null;
  content: JSONContent;
  text: string;
  fontMapper?: FontMapper;
  layoutMode?: PostLayoutMode | null;
  pageLayout?: {
    width: number;
    height: number;
    marginTop: number;
    marginBottom: number;
    marginLeft: number;
    marginRight: number;
  };
}): Promise<Uint8Array> {
  const { content, fontMapper, layoutMode, pageLayout } = params;

  const imageCache = await downloadAllImages(content);

  const children = content.content ? convertJSONContentToChildren(content, { fontMapper, pageLayout, imageCache }) : [];

  const sectionProperties =
    layoutMode === PostLayoutMode.PAGE && pageLayout
      ? {
          page: {
            size: {
              width: Math.round(mmToTwips(pageLayout.width)),
              height: Math.round(mmToTwips(pageLayout.height)),
            },
            margin: {
              top: Math.round(mmToTwips(pageLayout.marginTop)),
              bottom: Math.round(mmToTwips(pageLayout.marginBottom)),
              left: Math.round(mmToTwips(pageLayout.marginLeft)),
              right: Math.round(mmToTwips(pageLayout.marginRight)),
            },
          },
        }
      : {};

  const doc = new Document({
    styles: {
      default: {
        document: {
          run: {
            font: defaultValues.fontFamily,
            size: pxToHalfPt(defaultValues.fontSize),
          },
        },
      },
      paragraphStyles: [
        {
          id: 'Normal',
          name: 'Normal',
          basedOn: 'Normal',
          next: 'Normal',
          quickFormat: true,
          run: {
            font: defaultValues.fontFamily,
          },
        },
        {
          id: 'ImageCenter',
          name: 'Image Center',
          basedOn: 'Normal',
          next: 'Normal',
          quickFormat: true,
          paragraph: {
            alignment: AlignmentType.CENTER,
          },
        },
      ],
    },
    sections: [
      {
        properties: sectionProperties,
        children,
      },
    ],
  });

  const buffer = await Packer.toBuffer(doc);
  return new Uint8Array(buffer);
}

export function convertJSONContentToChildren(
  content: JSONContent,
  { fontMapper, pageLayout, imageCache, depth, baseIndent }: ConvertOptions = {},
): (Paragraph | Table)[] {
  const children: (Paragraph | Table)[] = [];

  if (!content.content) {
    return children;
  }

  const bodyNode = content.content?.find((node) => node.type === 'body');
  const nodesToProcess = bodyNode?.content || content.content;

  const bodyAttrs = {
    paragraphIndent: bodyNode?.attrs?.paragraphIndent || 0, // rem
    blockGap: bodyNode?.attrs?.blockGap || 0, // rem
  };

  const options = { fontMapper, bodyAttrs, pageLayout, imageCache, depth, baseIndent };
  for (const node of nodesToProcess) {
    const converted = match(node.type)
      .with('paragraph', () => [convertParagraph(node, options)])
      .with('bullet_list', () => convertListToParagraphs(node, false, options))
      .with('ordered_list', () => convertListToParagraphs(node, true, options))
      .with('blockquote', () => convertBlockquoteToParagraphs(node, options))
      .with('callout', () => convertCalloutToParagraphs(node, options))
      .with('code_block', () => [convertCodeBlock(node, options)])
      .with('horizontal_rule', () => [convertHorizontalRule(options)])
      .with('page_break', () => [
        new Paragraph({
          children: [new PageBreak()],
        }),
      ])
      .with('fold', () => convertFold(node, options))
      .with('table', () => [convertTable(node, options)])
      .with('image', () => [convertImage(node, options)])
      .with('embed', () => [convertEmbed(node, options)])
      .with('file', () => [convertFile(node, options)])
      .with('html_block', () => [convertHtmlBlock(node, options)])
      .otherwise(() => []);

    children.push(...converted);
  }

  return children;
}
