export const formatDuration = (totalSeconds: number): string => {
  const seconds = Math.max(0, Math.round(totalSeconds));
  if (seconds < 60) return `${seconds}초`;
  const minutes = Math.floor(seconds / 60);
  if (minutes < 60) return `${minutes}분 ${seconds % 60}초`;
  return `${Math.floor(minutes / 60)}시간 ${minutes % 60}분`;
};
