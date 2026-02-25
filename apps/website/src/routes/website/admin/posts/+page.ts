import { ADMIN_ITEMS_PER_PAGE } from '@typie/ui/constants';
import { loadQuery } from '$lib/graphql';
import { graphql } from '$mearie';

export const load = async (event) => {
  const { url } = event;
  const searchParams = url.searchParams;
  const page = Number(searchParams.get('page') ?? '1');
  const search = searchParams.get('search') ?? undefined;
  const offset = (page - 1) * ADMIN_ITEMS_PER_PAGE;
  return {
    query: await loadQuery(
      event,
      graphql(`
        query AdminPosts_Query($search: String, $offset: Int!, $limit: Int!) {
          adminPosts(search: $search, offset: $offset, limit: $limit) {
            totalCount

            posts {
              id
              title
              subtitle
              type
              createdAt
              updatedAt
              contentRating
              excerpt
              reactionCount
              characterCount
              entity {
                id
                slug
                visibility
                state
                ancestors {
                  id
                  node {
                    __typename
                    ... on Folder {
                      name
                    }
                    ... on Post {
                      title
                    }
                    ... on Document {
                      title
                    }
                  }
                }
                user {
                  id
                  name
                  email
                  avatar {
                    id
                    url
                  }
                }
              }

              coverImage {
                id
                url
              }
            }
          }
        }
      `),
      {
        search,
        offset,
        limit: ADMIN_ITEMS_PER_PAGE,
      },
    ),
  };
};
