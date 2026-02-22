import ky from 'ky';
import { env } from '@/env';

type CreateIssueParams = { title: string; description: string; labelIds?: string[] };
export const createIssue = async ({ title, description, labelIds }: CreateIssueParams) => {
  await ky.post('https://api.linear.app/graphql', {
    headers: { Authorization: env.LINEAR_API_KEY },
    json: {
      query: `mutation($input: IssueCreateInput!) { issueCreate(input: $input) { success } }`,
      variables: {
        input: {
          teamId: env.LINEAR_TEAM_ID,
          title,
          description,
          ...(labelIds && labelIds.length > 0 && { labelIds }),
        },
      },
    },
  });
};
