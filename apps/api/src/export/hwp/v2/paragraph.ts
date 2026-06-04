// spell-checker:words inlines
import { resolveRubyCharShape } from '../styles.ts';
import { resolveCharShapeV2 } from './styles.ts';
import type { Inline, ParagraphV2 } from '../../core/v2/types.ts';
import type { HwpConvertContext, InlineSegment } from '../types.ts';

export type ParagraphPartV2 = { segments: InlineSegment[]; pageBreakAfter: boolean };

function segmentForInline(inline: Inline, ctx: HwpConvertContext): InlineSegment | undefined {
  switch (inline.type) {
    case 'run': {
      const { text, style } = inline.run;
      const charShapeId = resolveCharShapeV2(style, ctx);
      const rubyCharShapeId = style.ruby ? resolveRubyCharShape(charShapeId, ctx) : undefined;
      return { text, charShapeId, link: style.link, ruby: style.ruby, rubyCharShapeId };
    }
    case 'hard_break': {
      return { text: '\n', charShapeId: ctx.defaultCharShapeId };
    }
    case 'tab': {
      return { text: '\t', charShapeId: ctx.defaultCharShapeId };
    }
    case 'page_break': {
      return undefined;
    }
  }
}

export function segmentsFromParagraphV2(p: ParagraphV2, ctx: HwpConvertContext): InlineSegment[] {
  const segments: InlineSegment[] = [];
  for (const inline of p.inlines) {
    const seg = segmentForInline(inline, ctx);
    if (seg) segments.push(seg);
  }
  return segments;
}

export function splitParagraphPartsV2(p: ParagraphV2, ctx: HwpConvertContext): ParagraphPartV2[] {
  const parts: ParagraphPartV2[] = [];
  let current: InlineSegment[] = [];
  for (const inline of p.inlines) {
    if (inline.type === 'page_break') {
      parts.push({ segments: current, pageBreakAfter: true });
      current = [];
      continue;
    }
    const seg = segmentForInline(inline, ctx);
    if (seg) current.push(seg);
  }
  parts.push({ segments: current, pageBreakAfter: false });
  return parts;
}
