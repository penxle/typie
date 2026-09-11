export type ChangelogEntry = {
  id: string;
  title: string;
  date: string;
  image: { url: string } | null;
  body: string;
};

export type ChangelogPage = { entries: ChangelogEntry[]; hasMore: boolean };

export const COPY = {
  pageTitle: '업데이트 노트',
  description: '올인원 글쓰기 도구 타이피의 새로운 기능과 개선 사항들을 확인해보세요.',
  title: ['새로운 기능,', '더 나은 경험.'] as const,
  sub: '타이피의 새로운 기능과 개선 사항들을 확인해보세요.',
  failed: '업데이트 노트를 불러오지 못했어요',
} as const;

export const formatDate = (date: string): string => date.slice(0, 10).replaceAll('-', '. ');

export const fetchPage = async (page: number, fetcher: typeof fetch = fetch): Promise<ChangelogPage | null> => {
  try {
    const response = await fetcher(`/api/changelog?page=${page}`);
    if (!response.ok) return null;
    return (await response.json()) as ChangelogPage;
  } catch {
    return null;
  }
};
