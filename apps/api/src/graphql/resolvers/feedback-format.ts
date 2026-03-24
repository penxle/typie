import dedent from 'dedent';

const moodLabels: Record<string, string> = {
  angry: '😠 불만',
  annoyed: '😟 아쉬움',
  good: '🙂 만족',
  great: '😄 매우 만족',
};

type FeedbackUser = {
  id: string;
  name: string;
  email: string;
};

type FormatFeedbackDescriptionParams = {
  content: string;
  user: FeedbackUser;
  mood?: string | null;
  url?: string | null;
  platform?: string | null;
  osVersion?: string | null;
  appVersion?: string | null;
  deviceName?: string | null;
};

const none = '(없음)';

function formatOptionalText(value?: string | null): string {
  return value?.trim() || none;
}

function formatMood(mood?: string | null): string {
  if (!mood) return none;
  return moodLabels[mood] ?? mood;
}

export function formatFeedbackDescription({
  content,
  user,
  mood,
  url,
  platform,
  osVersion,
  appVersion,
  deviceName,
}: FormatFeedbackDescriptionParams): string {
  return dedent`
    ${content}

    ---

    - **사용자:** ${user.name} (${user.email})
    - **사용자 ID:** ${user.id}
    - **기분:** ${formatMood(mood)}
    - **페이지:** ${formatOptionalText(url)}
    - **플랫폼:** ${formatOptionalText(platform)}
    - **OS 버전:** ${formatOptionalText(osVersion)}
    - **앱 버전:** ${formatOptionalText(appVersion)}
    - **기기:** ${formatOptionalText(deviceName)}
  `;
}
