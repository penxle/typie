import { error } from '@sveltejs/kit';
import { gql, GraphQLClient } from 'graphql-request';
import { env } from '$env/dynamic/public';
import { isLegalSlug, LEGAL_DOCUMENTS } from './legal';
import type { PageServerLoad } from './$types';
import type { LegalDocument } from './legal';

const QUERY = gql`
  query GetLegalDocument($stage: Stage!, $slug: String!) {
    document(where: { slug: $slug }, stage: $stage) {
      title
      body
    }
  }
`;

export const load: PageServerLoad = async ({ params, fetch }) => {
  const slug = params.slug;
  if (!isLegalSlug(slug)) error(404);

  const client = new GraphQLClient(env.PUBLIC_CMS_URL, { fetch });
  const data = await client.request<{ document: LegalDocument | null }>(QUERY, { slug, stage: env.PUBLIC_CMS_STAGE });
  const document = data.document;
  if (!document) error(404);

  return { document, description: LEGAL_DOCUMENTS[slug].description };
};
