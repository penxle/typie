export type LineDiffEntry = { type: 'same' | 'add' | 'del'; line: string };

// 빈 문자열은 "빈 줄 1개"가 아니라 "줄 없음"으로 취급한다 (diffLines('', '') === []).
const toLines = (text: string): string[] => (text === '' ? [] : text.split('\n'));

// LCS(최장 공통 부분열) 기반 라인 diff. 문서 하나 분량(수백 줄) 기준이라 O(n*m) DP로 충분하다.
export const diffLines = (a: string, b: string): LineDiffEntry[] => {
  const linesA = toLines(a);
  const linesB = toLines(b);
  const n = linesA.length;
  const m = linesB.length;

  // dp[i][j] = LCS length of linesA[i:] and linesB[j:]
  const dp: number[][] = Array.from({ length: n + 1 }, () => Array.from({ length: m + 1 }, () => 0));
  for (let i = n - 1; i >= 0; i--) {
    for (let j = m - 1; j >= 0; j--) {
      dp[i][j] = linesA[i] === linesB[j] ? dp[i + 1][j + 1] + 1 : Math.max(dp[i + 1][j], dp[i][j + 1]);
    }
  }

  const result: LineDiffEntry[] = [];
  let i = 0;
  let j = 0;
  while (i < n && j < m) {
    if (linesA[i] === linesB[j]) {
      result.push({ type: 'same', line: linesA[i] });
      i++;
      j++;
    } else if (dp[i + 1][j] >= dp[i][j + 1]) {
      result.push({ type: 'del', line: linesA[i] });
      i++;
    } else {
      result.push({ type: 'add', line: linesB[j] });
      j++;
    }
  }
  while (i < n) {
    result.push({ type: 'del', line: linesA[i] });
    i++;
  }
  while (j < m) {
    result.push({ type: 'add', line: linesB[j] });
    j++;
  }

  return result;
};
