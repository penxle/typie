import * as aws from '@pulumi/aws';
import { zones } from '$aws/route53';

// spell-checker:words sesv2

export const configurationSet = new aws.sesv2.ConfigurationSet('glttr.io', {
  configurationSetName: 'glttr_io',
});

export const emailIdentity = new aws.sesv2.EmailIdentity('glttr.io', {
  emailIdentity: 'glttr.io',
  configurationSetName: configurationSet.configurationSetName,
});

new aws.sesv2.EmailIdentityMailFromAttributes('glttr.io', {
  emailIdentity: emailIdentity.id,
  mailFromDomain: 'mail.glttr.io',
});

emailIdentity.dkimSigningAttributes.tokens.apply((tokens) => {
  for (const token of tokens) {
    new aws.route53.Record(`${token}._domainkey.glttr.io`, {
      zoneId: zones.glttr_io.zoneId,
      type: 'CNAME',
      name: `${token}._domainkey.glttr.io`,
      records: [`${token}.dkim.amazonses.com`],
      ttl: 300,
    });
  }
});

new aws.route53.Record('mail.glttr.io|mx', {
  zoneId: zones.glttr_io.zoneId,
  type: 'MX',
  name: 'mail.glttr.io',
  records: ['10 feedback-smtp.ap-northeast-2.amazonses.com'],
  ttl: 300,
});

new aws.route53.Record('mail.glttr.io|txt', {
  zoneId: zones.glttr_io.zoneId,
  type: 'TXT',
  name: 'mail.glttr.io',
  records: ['v=spf1 include:amazonses.com ~all'],
  ttl: 300,
});

new aws.route53.Record('_dmarc.glttr.io|txt', {
  zoneId: zones.glttr_io.zoneId,
  type: 'TXT',
  name: '_dmarc.glttr.io',
  records: ['v=DMARC1; p=none;'],
  ttl: 300,
});

export const outputs = {
  AWS_SES_CONFIGURATION_SET: configurationSet.arn,
  AWS_SES_EMAIL_IDENTITY: emailIdentity.arn,
};
