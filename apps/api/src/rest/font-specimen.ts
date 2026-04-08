import { HTTPException } from 'hono/http-exception';
import { applySvgRootColor } from '#/utils/color.ts';

type RenderFontSpecimenSvgParams = {
  text: string;
  fallbacks?: string[];
  color?: string | null;
  renderTextToSvg: (text: string) => Promise<string>;
};

export function normalizeSpecimenFallbacks(primaryText: string, fallbacks: readonly string[]): string[] {
  const normalizedPrimary = primaryText.trim().toLowerCase();
  const seen = new Set<string>([normalizedPrimary]);

  return fallbacks
    .map((fallback) => fallback.trim())
    .filter((fallback) => fallback.length > 0)
    .filter((fallback) => {
      const normalizedFallback = fallback.toLowerCase();
      if (seen.has(normalizedFallback)) {
        return false;
      }

      seen.add(normalizedFallback);
      return true;
    });
}

export async function renderFontSpecimenSvg({
  text,
  fallbacks = [],
  color = null,
  renderTextToSvg,
}: RenderFontSpecimenSvgParams): Promise<string> {
  for (const candidate of [text, ...normalizeSpecimenFallbacks(text, fallbacks)]) {
    try {
      const svg = await renderTextToSvg(candidate);
      return applySvgRootColor(svg, color);
    } catch (err) {
      if (String(err).includes('missing glyph')) {
        continue;
      }

      throw err;
    }
  }

  throw new HTTPException(422);
}
