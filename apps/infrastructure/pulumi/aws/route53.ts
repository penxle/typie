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
};

new aws.route53.Record('glitter.im|txt', {
  zoneId: zones.glitter_im.zoneId,
  type: 'TXT',
  name: 'glitter.im',
  records: [
    // spell-checker:disable-next-line
    'google-site-verification=QXQUl_XRbKvl20qm2ClmqOcRpsdVPQVHd7RT9CLYtsE',
  ],
  ttl: 300,
});

new aws.route53.Record('send.glitter.im|mx', {
  zoneId: zones.glitter_im.zoneId,
  type: 'MX',
  name: 'send.glitter.im',
  records: ['10 feedback-smtp.ap-northeast-1.amazonses.com'],
  ttl: 300,
});

new aws.route53.Record('send.glitter.im|txt', {
  zoneId: zones.glitter_im.zoneId,
  type: 'TXT',
  name: 'send.glitter.im',
  records: ['v=spf1 include:amazonses.com ~all'],
  ttl: 300,
});

new aws.route53.Record('resend._domainkey.glitter.im|txt', {
  zoneId: zones.glitter_im.zoneId,
  type: 'TXT',
  name: 'resend._domainkey.glitter.im',
  records: [
    'p=MIGfMA0GCSqGSIb3DQEBAQUAA4GNADCBiQKBgQD1vooTTa2V08o1vMw003MVvE6/UWJ8NlYrOEYyTKGP4kKXr2yXXeXfCmI9r4NmDJROUd1kz6Zg4dorcCDe4ai6kgI9iE+jSrp9qwCV/3Jxi/lu8yC6kNhYCPHrl7twgor7kq6cIoZFzuDbDQXqXJlmFJB9siv+fEYMzW48x0IcDQIDAQAB',
  ],
  ttl: 300,
});
