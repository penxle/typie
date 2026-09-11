export const LEGAL_DOCUMENTS = {
  terms: { description: '타이피 서비스 이용약관' },
  privacy: { description: '타이피 개인정보처리방침' },
} as const;

export type LegalSlug = keyof typeof LEGAL_DOCUMENTS;

export type LegalDocument = { title: string; body: string };

export const isLegalSlug = (slug: string): slug is LegalSlug => Object.hasOwn(LEGAL_DOCUMENTS, slug);
