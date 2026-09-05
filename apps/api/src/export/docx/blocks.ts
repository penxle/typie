import {
  AlignmentType,
  BorderStyle,
  ImportedXmlComponent,
  LineRuleType,
  PageBreak,
  Paragraph,
  ShadingType,
  Table,
  TableCell,
  TableRow,
  TextRun,
  WidthType,
} from 'docx';
import { resolveCalloutFillToHex, resolveColorToHex } from '../core/theme.ts';
import type { IParagraphOptions, ISpacingProperties } from 'docx';

export type FileChild = Paragraph | Table;

export const NO_BORDER = { style: BorderStyle.NONE, size: 0 } as const;
export const SUBTLE_BORDER = { style: BorderStyle.SINGLE, size: 1, color: 'E3E4EB' } as const;

/** line_height(× 100)를 AT_LEAST 모드의 twips로 변환. fontSizePt 기준 절대값. */
function lineHeightToSpacing(lineHeight: number | undefined, fontSizePt: number, afterTwips?: number): ISpacingProperties | undefined {
  if (!lineHeight && afterTwips == null) return undefined;
  if (lineHeight) {
    return { line: Math.round(fontSizePt * (lineHeight / 100) * 20), lineRule: LineRuleType.AT_LEAST, after: afterTwips };
  }
  return { after: afterTwips };
}

function mapAlignment(align: string): (typeof AlignmentType)[keyof typeof AlignmentType] | undefined {
  switch (align) {
    case 'left': {
      return AlignmentType.START;
    }
    case 'center': {
      return AlignmentType.CENTER;
    }
    case 'right': {
      return AlignmentType.END;
    }
    case 'justify': {
      return AlignmentType.BOTH;
    }
    default: {
      return undefined;
    }
  }
}

export type DocDefaults = { fontSizePt: number; blockGapTwips: number };
const DEFAULT_DEFAULTS: DocDefaults = { fontSizePt: 12, blockGapTwips: 240 };

export function convertParagraph(
  node: { align?: string; line_height?: number },
  inlineChildren: IParagraphOptions['children'],
  indentTwips = 0,
  defaults: DocDefaults = DEFAULT_DEFAULTS,
): Paragraph {
  const alignment = node.align ? mapAlignment(node.align) : undefined;
  const spacing = lineHeightToSpacing(node.line_height, defaults.fontSizePt, defaults.blockGapTwips);

  return new Paragraph({
    alignment,
    spacing,
    indent: indentTwips ? { firstLine: indentTwips } : undefined,
    children: inlineChildren ?? [],
  });
}

export function convertCallout(node: { variant?: string }, innerElements: FileChild[]): Table {
  const variant = node.variant ?? 'info';
  const colorKey = `ui.callout.${variant}`;
  const hex = resolveColorToHex(colorKey);
  const bgFill = resolveCalloutFillToHex(variant) ?? 'F3F4F6';
  const borderColor = hex ?? 'CCCCCC';

  const cellChildren = innerElements.filter((el): el is Paragraph => el instanceof Paragraph);
  if (cellChildren.length === 0) {
    cellChildren.push(new Paragraph({ children: [] }));
  }

  return new Table({
    rows: [
      new TableRow({
        children: [
          new TableCell({
            children: cellChildren,
            shading: { fill: bgFill, type: ShadingType.CLEAR },
            borders: {
              top: { style: BorderStyle.SINGLE, size: 1, color: borderColor },
              bottom: { style: BorderStyle.SINGLE, size: 1, color: borderColor },
              left: { style: BorderStyle.SINGLE, size: 24, color: borderColor },
              right: { style: BorderStyle.SINGLE, size: 1, color: borderColor },
            },
            margins: { top: 80, bottom: 80, left: 120, right: 120 },
          }),
        ],
      }),
    ],
    width: { size: 100, type: WidthType.PERCENTAGE },
  });
}

export function convertHorizontalRule(): Paragraph {
  const rect = new ImportedXmlComponent('v:rect', {
    style: 'width:0;height:1.5pt',
    'o:hralign': 'center', // cspell:disable-line
    'o:hrstd': 't', // cspell:disable-line
    'o:hr': 't',
    fillcolor: '#cccccc',
    stroked: 'f',
  });
  const pict = new ImportedXmlComponent('w:pict');
  pict.push(rect);
  const run = new ImportedXmlComponent('w:r');
  run.push(pict);

  return new Paragraph({
    spacing: { before: 120, after: 120 },
    children: [run],
  });
}

export function convertPageBreak(): Paragraph {
  return new Paragraph({
    children: [new PageBreak()],
  });
}

export function convertListItemParagraph(
  inlineChildren: IParagraphOptions['children'],
  listType: 'bullet' | 'ordered',
  level: number,
  numberingRef: string,
  paragraphNode?: { align?: string; line_height?: number },
  defaults: DocDefaults = DEFAULT_DEFAULTS,
): Paragraph {
  const alignment = paragraphNode?.align ? mapAlignment(paragraphNode.align) : undefined;
  const spacing = lineHeightToSpacing(paragraphNode?.line_height, defaults.fontSizePt, defaults.blockGapTwips);

  if (listType === 'bullet') {
    return new Paragraph({
      alignment,
      spacing,
      bullet: { level },
      children: inlineChildren ?? [],
    });
  }

  return new Paragraph({
    alignment,
    spacing,
    numbering: { reference: numberingRef, level },
    children: inlineChildren ?? [],
  });
}

export function convertPlaceholderTable(text: string): Table {
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
                children: [new TextRun({ text, color: '999999' })],
              }),
            ],
            shading: { fill: 'F3F4F6', type: ShadingType.CLEAR },
            borders: { top: SUBTLE_BORDER, bottom: SUBTLE_BORDER, left: SUBTLE_BORDER, right: SUBTLE_BORDER },
            margins: { top: 100, bottom: 100, left: 160, right: 160 },
          }),
        ],
      }),
    ],
  });
}

export function toBlockChildren(elements: FileChild[]): (Paragraph | Table)[] {
  const result = elements.filter((el): el is Paragraph | Table => el instanceof Paragraph || el instanceof Table);
  if (result.length === 0) result.push(new Paragraph({ children: [] }));
  return result;
}
