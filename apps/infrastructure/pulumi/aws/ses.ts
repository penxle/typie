import * as aws from '@pulumi/aws';
import { zones } from '$aws/route53';

// spell-checker:words sesv2

export const configurationSet = new aws.sesv2.ConfigurationSet('glitter.rocks', {
  configurationSetName: 'glitter_rocks',
});

export const emailIdentity = new aws.sesv2.EmailIdentity('glitter.rocks', {
  emailIdentity: 'glitter.rocks',
  configurationSetName: configurationSet.configurationSetName,
});

new aws.sesv2.EmailIdentityMailFromAttributes('glitter.rocks', {
  emailIdentity: emailIdentity.id,
  mailFromDomain: 'mail.glitter.rocks',
});

emailIdentity.dkimSigningAttributes.tokens.apply((tokens) => {
  for (const token of tokens) {
    new aws.route53.Record(`${token}._domainkey.glitter.rocks`, {
      zoneId: zones.glitter_rocks.zoneId,
      type: 'CNAME',
      name: `${token}._domainkey.glitter.rocks`,
      records: [`${token}.dkim.amazonses.com`],
      ttl: 300,
    });
  }
});

new aws.route53.Record('mail.glitter.rocks|mx', {
  zoneId: zones.glitter_rocks.zoneId,
  type: 'MX',
  name: 'mail.glitter.rocks',
  records: ['10 feedback-smtp.ap-northeast-2.amazonses.com'],
  ttl: 300,
});

new aws.route53.Record('mail.glitter.rocks|txt', {
  zoneId: zones.glitter_rocks.zoneId,
  type: 'TXT',
  name: 'mail.glitter.rocks',
  records: ['v=spf1 include:amazonses.com ~all'],
  ttl: 300,
});

new aws.route53.Record('_dmarc.glitter.rocks|txt', {
  zoneId: zones.glitter_rocks.zoneId,
  type: 'TXT',
  name: '_dmarc.glitter.rocks',
  records: ['v=DMARC1; p=none;'],
  ttl: 300,
});

export const outputs = {
  AWS_SES_CONFIGURATION_SET: configurationSet.arn,
  AWS_SES_EMAIL_IDENTITY: emailIdentity.arn,
};
