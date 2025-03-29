import * as aws from '@pulumi/aws';
import { zones } from '$aws/route53';

// spell-checker:words sesv2

export const configurationSet = new aws.sesv2.ConfigurationSet('typie_co', {
  configurationSetName: 'typie_co',
});

export const emailIdentity = new aws.sesv2.EmailIdentity('typie.co', {
  emailIdentity: 'typie.co',
  configurationSetName: configurationSet.configurationSetName,
});

new aws.sesv2.EmailIdentityMailFromAttributes('typie.co', {
  emailIdentity: emailIdentity.id,
  mailFromDomain: 'mail.typie.co',
});

emailIdentity.dkimSigningAttributes.tokens.apply((tokens) => {
  for (const token of tokens) {
    new aws.route53.Record(`${token}._domainkey.typie.co`, {
      zoneId: zones.typie_co.zoneId,
      type: 'CNAME',
      name: `${token}._domainkey.typie.co`,
      records: [`${token}.dkim.amazonses.com`],
      ttl: 300,
    });
  }
});

new aws.route53.Record('mail.typie.co|mx', {
  zoneId: zones.typie_co.zoneId,
  type: 'MX',
  name: 'mail.typie.co',
  records: ['10 feedback-smtp.ap-northeast-2.amazonses.com'],
  ttl: 300,
});

new aws.route53.Record('mail.typie.co|txt', {
  zoneId: zones.typie_co.zoneId,
  type: 'TXT',
  name: 'mail.typie.co',
  records: ['v=spf1 include:amazonses.com ~all'],
  ttl: 300,
});

new aws.route53.Record('_dmarc.typie.co|txt', {
  zoneId: zones.typie_co.zoneId,
  type: 'TXT',
  name: '_dmarc.typie.co',
  records: ['v=DMARC1; p=none;'],
  ttl: 300,
});

export const outputs = {
  AWS_SES_CONFIGURATION_SET: configurationSet.arn,
  AWS_SES_EMAIL_IDENTITY: emailIdentity.arn,
};
