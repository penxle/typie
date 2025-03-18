import * as aws from '@pulumi/aws';
import { zones } from '$aws/route53';

// spell-checker:words sesv2

export const configurationSet = new aws.sesv2.ConfigurationSet('glitter.im', {
  configurationSetName: 'glitter_im',
});

export const emailIdentity = new aws.sesv2.EmailIdentity('glitter.im', {
  emailIdentity: 'glitter.im',
  configurationSetName: configurationSet.configurationSetName,
});

new aws.sesv2.EmailIdentityMailFromAttributes('glitter.im', {
  emailIdentity: emailIdentity.id,
  mailFromDomain: 'mail.glitter.im',
});

emailIdentity.dkimSigningAttributes.tokens.apply((tokens) => {
  for (const token of tokens) {
    new aws.route53.Record(`${token}._domainkey.glitter.im`, {
      zoneId: zones.glitter_im.zoneId,
      type: 'CNAME',
      name: `${token}._domainkey.glitter.im`,
      records: [`${token}.dkim.amazonses.com`],
      ttl: 300,
    });
  }
});

new aws.route53.Record('mail.glitter.im|mx', {
  zoneId: zones.glitter_im.zoneId,
  type: 'MX',
  name: 'mail.glitter.im',
  records: ['10 feedback-smtp.ap-northeast-2.amazonses.com'],
  ttl: 300,
});

new aws.route53.Record('mail.glitter.im|txt', {
  zoneId: zones.glitter_im.zoneId,
  type: 'TXT',
  name: 'mail.glitter.im',
  records: ['v=spf1 include:amazonses.com ~all'],
  ttl: 300,
});

new aws.route53.Record('_dmarc.glitter.im|txt', {
  zoneId: zones.glitter_im.zoneId,
  type: 'TXT',
  name: '_dmarc.glitter.im',
  records: ['v=DMARC1; p=none;'],
  ttl: 300,
});

export const outputs = {
  AWS_SES_CONFIGURATION_SET: configurationSet.arn,
  AWS_SES_EMAIL_IDENTITY: emailIdentity.arn,
};
