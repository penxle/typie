export const ISSUE_STATUSES = [
  { value: 'OPEN', label: '열림', colorToken: 'text.faint' },
  { value: 'IN_PROGRESS', label: '진행중', colorToken: 'accent.warning.default' },
  { value: 'RESOLVED', label: '해결됨', colorToken: 'accent.brand.default' },
  { value: 'CLOSED', label: '닫힘', colorToken: 'text.disabled' },
] as const;

export const ISSUE_PRIORITIES = [
  { value: 'NONE', label: '우선순위 없음', colorToken: 'text.muted', subtleToken: 'surface.muted' },
  { value: 'URGENT', label: '긴급', colorToken: 'accent.warning.default', subtleToken: 'accent.warning.subtle' },
  { value: 'HIGH', label: '높음', colorToken: 'accent.danger.default', subtleToken: 'accent.danger.subtle' },
  { value: 'MEDIUM', label: '중간', colorToken: 'accent.warning.default', subtleToken: 'accent.warning.subtle' },
  { value: 'LOW', label: '낮음', colorToken: 'accent.info.default', subtleToken: 'accent.info.subtle' },
] as const;

export type IssueStatus = (typeof ISSUE_STATUSES)[number]['value'];
export type IssuePriority = (typeof ISSUE_PRIORITIES)[number]['value'];

export const getStatusMeta = (status: IssueStatus) => ISSUE_STATUSES.find((s) => s.value === status) ?? ISSUE_STATUSES[0];

export const getPriorityMeta = (priority: IssuePriority) => ISSUE_PRIORITIES.find((p) => p.value === priority) ?? ISSUE_PRIORITIES[0];
