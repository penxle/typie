import { SendEmailCommand } from '@aws-sdk/client-ses';
import { render } from '@react-email/components';
import * as aws from '@/external/aws';
import type * as React from 'react';

type SendEmailParams = {
  subject: string;
  recipient: string;
  body: React.ReactElement;
};

export const sendEmail = async ({ subject, recipient, body }: SendEmailParams) => {
  await aws.ses.send(
    new SendEmailCommand({
      Source: 'typie <hello@typie.co>',
      Destination: {
        ToAddresses: [recipient],
      },
      Message: {
        Subject: {
          Data: subject,
        },
        Body: {
          Html: {
            Data: await render(body),
          },
        },
      },
    }),
  );
};
