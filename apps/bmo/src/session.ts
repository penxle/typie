import { DynamoDBClient } from '@aws-sdk/client-dynamodb';
import { DynamoDBDocumentClient, GetCommand, PutCommand } from '@aws-sdk/lib-dynamodb';

const client = DynamoDBDocumentClient.from(new DynamoDBClient({}));
const TABLE_NAME = 'bmo-sessions';
const SESSION_TTL = 60 * 60 * 24 * 7;

export const getSession = async (threadKey: string): Promise<string | null> => {
  const result = await client.send(new GetCommand({ TableName: TABLE_NAME, Key: { threadKey } }));
  if (!result.Item) return null;
  return result.Item.sessionId as string;
};

export const setSession = async (threadKey: string, sessionId: string): Promise<void> => {
  await client.send(
    new PutCommand({
      TableName: TABLE_NAME,
      Item: {
        threadKey,
        sessionId,
        ttl: Math.floor(Date.now() / 1000) + SESSION_TTL,
      },
    }),
  );
};
