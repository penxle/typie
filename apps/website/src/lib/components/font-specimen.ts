export function familySpecimenFallbacks(displayName: string, familyName: string): string[] {
  return normalizeSpecimenFallbacks(displayName, [familyName]);
}

export function weightSpecimenFallbacks(label: string, subfamilyDisplayName: string | null | undefined, weight: number): string[] {
  return normalizeSpecimenFallbacks(label, [subfamilyDisplayName ?? '', String(weight)]);
}

export function buildFontSpecimenSearchParams(text: string, fallbacks: readonly string[] = []): URLSearchParams {
  const searchParams = new URLSearchParams({ text });

  for (const fallback of normalizeSpecimenFallbacks(text, fallbacks)) {
    searchParams.append('fallbacks', fallback);
  }

  return searchParams;
}

export function buildFontSpecimenUrl(apiUrl: string, fontId: string, text: string, fallbacks: readonly string[] = []): string {
  const url = new URL(`/font/${fontId}/specimen`, apiUrl);

  for (const [key, value] of buildFontSpecimenSearchParams(text, fallbacks)) {
    url.searchParams.append(key, value);
  }

  return url.href;
}

export function buildFontSpecimenCacheKey(fontId: string, text: string, fallbacks: readonly string[] = []): string {
  return JSON.stringify([fontId, text, normalizeSpecimenFallbacks(text, fallbacks)]);
}

function normalizeSpecimenFallbacks(primaryText: string, fallbacks: readonly string[]): string[] {
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
