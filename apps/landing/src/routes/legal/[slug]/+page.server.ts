import { error } from '@sveltejs/kit';
import { isLegalSlug, LEGAL_DOCUMENTS } from './legal';
import type { PageServerLoad } from './$types';
import type { LegalDocument } from './legal';

const CMS_URL = 'https://ap-northeast-1.cdn.hygraph.com/content/clmrdf5dq1rhd01t946f6a2d5/master';
const QUERY = 'query ($slug: String!) { document(where: { slug: $slug }, stage: PUBLISHED) { title body } }';

export const load: PageServerLoad = async ({ params, fetch }) => {
  const slug = params.slug;
  if (!isLegalSlug(slug)) error(404);

  const response = await fetch(CMS_URL, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ query: QUERY, variables: { slug } }),
  });
  const json = (await response.json()) as { data?: { document: LegalDocument | null } };
  const document = json.data?.document;
  if (!document) error(404);

  return { document, description: LEGAL_DOCUMENTS[slug].description };
};
