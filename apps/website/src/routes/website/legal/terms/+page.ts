import { gql, GraphQLClient } from 'graphql-request';
import { env } from '$env/dynamic/public';

const query = gql`
  query GetTermsDocument($stage: Stage!) {
    document(where: { slug: "terms" }, stage: $stage) {
      id
      title
      body
      updatedAt
    }
  }
`;

export const load = async ({ fetch }) => {
  const client = new GraphQLClient(env.PUBLIC_CMS_URL, { fetch });
  const stage = env.PUBLIC_CMS_STAGE;

  const data = await client.request<{
    document: {
      id: string;
      title: string;
      body: string;
      updatedAt: string;
    };
  }>(query, { stage });

  return {
    document: data.document,
  };
};
