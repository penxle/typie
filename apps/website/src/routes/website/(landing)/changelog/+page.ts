import { gql, GraphQLClient } from 'graphql-request';
import { env } from '$env/dynamic/public';

const query = gql`
  query GetChangelogs {
    changelogs(orderBy: date_DESC) {
      id
      title
      date
      image {
        url
      }
      body
    }
  }
`;

export const load = async ({ fetch }) => {
  const client = new GraphQLClient(env.PUBLIC_CMS_URL, { fetch });

  const data = await client.request<{
    changelogs: {
      id: string;
      title: string;
      date: string;
      image?: {
        url: string;
      };
      body: string;
    }[];
  }>(query);

  return {
    entries: data.changelogs,
  };
};
