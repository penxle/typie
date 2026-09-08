import { error } from '@sveltejs/kit';
import { gql, GraphQLClient } from 'graphql-request';
import { env } from '$env/dynamic/public';
import { fetchPage } from './changelog';
import type { PageServerLoad } from './$types';
import type { ChangelogEntry } from './changelog';

const PREVIEW_QUERY = gql`
  query GetChangelogPreview($stage: Stage!, $where: ChangelogWhereInput) {
    changelogs(stage: $stage, where: $where, first: 1) {
      id
      title
      date
      image {
        url(transformation: { document: { output: { format: autoImage } } })
      }
      body
    }
  }
`;

export const load: PageServerLoad = async ({ fetch, url }) => {
  const id = url.searchParams.get('id');
  if (!id) return { preview: false, initial: await fetchPage(1, fetch) };

  const client = new GraphQLClient(env.PUBLIC_CMS_URL, { fetch });
  const data = await client.request<{ changelogs: ChangelogEntry[] }>(PREVIEW_QUERY, { stage: env.PUBLIC_CMS_STAGE, where: { id } });

  const entry = data.changelogs[0];
  if (!entry) error(404);

  return { preview: true, initial: { entries: [{ ...entry, image: entry.image ?? null }], hasMore: false } };
};
