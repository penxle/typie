#!/usr/bin/env node

import { sendPushNotification } from '#/external/firebase.ts';

if (!process.argv[2] || !process.argv[3]) {
  console.error('Usage: node scripts/send-push-notification.ts <userId> <text>');
  process.exit(1);
}

const userId = process.argv[2];
const text = process.argv[3];

const success = await sendPushNotification({
  userId,
  title: '타이피',
  body: text,
});

if (success) {
  console.log(`Sent push notification to ${userId}`);
  process.exit(0);
} else {
  console.error(`Failed to send push notification to ${userId}`);
  process.exit(1);
}
