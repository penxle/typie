export const latestOfSpaceRuns = <T extends { space: { id: string } }>(items: readonly T[], previousSpaceId: string | null = null): T[] => {
  const leads: T[] = [];
  let lastSpaceId = previousSpaceId;
  for (const item of items) {
    if (item.space.id !== lastSpaceId) leads.push(item);
    lastSpaceId = item.space.id;
  }
  return leads;
};
