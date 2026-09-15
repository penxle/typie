export const folderCountLabel = (folderCount: number, publicationCount: number): string => {
  const parts: string[] = [];
  if (folderCount > 0) parts.push(`하위 시리즈 ${folderCount}개`);
  if (publicationCount > 0) parts.push(`글 ${publicationCount}개`);
  return parts.join(' · ');
};
