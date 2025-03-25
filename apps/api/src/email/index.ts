import { resend } from '@/external/resend';
import type * as React from 'react';

type SendEmailParams = {
  subject: string;
  recipient: string;
  body: React.ReactElement;
};

export const sendEmail = async ({ subject, recipient, body }: SendEmailParams) => {
  await resend.emails.send({
    from: '글리터 <hello@glitter.im>',
    to: [recipient],
    subject,
    react: body,
  });
};
