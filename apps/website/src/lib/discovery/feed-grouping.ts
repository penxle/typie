export type FeedGroup<T> = { lead: T; more: number };

export const groupConsecutiveBySpace = <T extends { space: { id: string } }>(items: readonly T[]): FeedGroup<T>[] => {
  const groups: FeedGroup<T>[] = [];
  for (const item of items) {
    const last = groups.at(-1);
    if (last && last.lead.space.id === item.space.id) {
      last.more += 1;
    } else {
      groups.push({ lead: item, more: 0 });
    }
  }
  return groups;
};
