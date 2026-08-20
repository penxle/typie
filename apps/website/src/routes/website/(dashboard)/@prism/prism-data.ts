import { mearieClient } from '$lib/graphql/client';
import { graphql } from '$mearie';
import type { ProjectedStreamFrame } from '@typie/prism';

const logQuery = graphql(`
  query DashboardLayout_PrismPanel_Log_Query($sessionId: ID!) {
    prismSessionLog(sessionId: $sessionId)
  }
`);

export const toFrame = (value: unknown): ProjectedStreamFrame => value as ProjectedStreamFrame;

export const fetchSessionLog = async (sessionId: string): Promise<ProjectedStreamFrame[]> => {
  const data = await mearieClient.query(logQuery, { sessionId }, { fetchPolicy: 'network-only' });
  return data.prismSessionLog.map(toFrame);
};
