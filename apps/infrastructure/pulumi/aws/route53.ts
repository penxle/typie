import * as aws from '@pulumi/aws';

const createZone = (domain: string) => {
  return new aws.route53.Zone(domain, {
    name: domain,
  });
};

export const zones = {
  glitter_im: createZone('glitter.im'),
  glitter_pizza: createZone('glitter.pizza'),
  glttr_io: createZone('glttr.io'),

  typie_co: createZone('typie.co'),
  typie_dev: createZone('typie.dev'),
  typie_me: createZone('typie.me'),
  typie_net: createZone('typie.net'),
  typie_io: createZone('typie.io'),
};

new aws.route53.Record('typie.co|txt', {
  zoneId: zones.typie_co.zoneId,
  type: 'TXT',
  name: 'typie.co',
  records: ['google-site-verification=Q-1ETLmF6p7XkzQM0wpDyF0wCBQREsjK1aZdxR-4ggQ'],
  ttl: 300,
});

new aws.route53.Record('send.typie.co|mx', {
  zoneId: zones.typie_co.zoneId,
  type: 'MX',
  name: 'send.typie.co',
  records: ['10 feedback-smtp.ap-northeast-1.amazonses.com'],
  ttl: 300,
});

new aws.route53.Record('send.typie.co|txt', {
  zoneId: zones.typie_co.zoneId,
  type: 'TXT',
  name: 'send.typie.co',
  records: ['v=spf1 include:amazonses.com ~all'],
  ttl: 300,
});

new aws.route53.Record('resend._domainkey.typie.co|txt', {
  zoneId: zones.typie_co.zoneId,
  type: 'TXT',
  name: 'resend._domainkey.typie.co',
  records: [
    'p=MIGfMA0GCSqGSIb3DQEBAQUAA4GNADCBiQKBgQC3+gXuQ+MyJT5PUzXZ7UJHY3vfwy41Bh6nmhBHbCDK7WJvr0COTx5LbwYH7+hZLIVLZgevUR4ErJO5w1GPAK29RRZ49iACcUgh4rJBME4l0w1h3vsq1guTkTjR2Uakrjx0r/dNof+XAMSvYJ0GMxF5CY1jFPJ/KmVPfQgROFUF5wIDAQAB',
  ],
  ttl: 300,
});
