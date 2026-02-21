export type SlackAppMentionEvent = {
  type: 'app_mention';
  user: string;
  text: string;
  ts: string;
  thread_ts?: string;
  channel: string;
  event_ts: string;
};

export type SlackEventWrapper = {
  type: 'event_callback';
  team_id: string;
  api_app_id: string;
  event: SlackAppMentionEvent;
  event_id: string;
  event_time: number;
};

export type SlackURLVerification = {
  type: 'url_verification';
  challenge: string;
};

export type SlackRequestBody = SlackEventWrapper | SlackURLVerification;
