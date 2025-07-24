import { gql, GraphQLClient } from 'graphql-request';
import { env } from '$env/dynamic/public';

const ITEMS_PER_PAGE = 5;

const query = gql`
  query GetChangelogs($stage: Stage!, $where: ChangelogWhereInput, $first: Int!, $skip: Int!) {
    changelogs(orderBy: date_DESC, stage: $stage, where: $where, first: $first, skip: $skip) {
      id
      title
      date
      image {
        url
      }
      body
    }
    changelogsConnection(where: $where, stage: $stage) {
      aggregate {
        count
      }
    }
  }
`;

export const load = async ({ fetch, url }) => {
  const client = new GraphQLClient(env.PUBLIC_CMS_URL, { fetch });

  const stage = env.PUBLIC_CMS_STAGE;
  const id = url.searchParams.get('id');
  const page = Number(url.searchParams.get('page')) || 1;

  const variables: {
    stage: string;
    where?: { id: string };
    first: number;
    skip: number;
  } = {
    stage,
    first: ITEMS_PER_PAGE,
    skip: (page - 1) * ITEMS_PER_PAGE,
  };

  if (id) {
    variables.where = { id };
  }

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
    changelogsConnection: {
      aggregate: {
        count: number;
      };
    };
  }>(query, variables);

  const totalPages = Math.ceil(data.changelogsConnection.aggregate.count / ITEMS_PER_PAGE);

  return {
    entries: data.changelogs,
    currentPage: page,
    totalPages,
    totalCount: data.changelogsConnection.aggregate.count,
    preview: !!id,
  };
};
