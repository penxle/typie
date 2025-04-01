import * as aws from '@pulumi/aws';

const createZone = (domain: string) => {
  return new aws.route53.Zone(domain, {
    name: domain,
  });
};

export const zones = {
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
  records: [
    'google-site-verification=Q-1ETLmF6p7XkzQM0wpDyF0wCBQREsjK1aZdxR-4ggQ',
    'google-site-verification=hZdtWP44my1tA-wUAvYlOKAAPSp2vHT6M5omQXCRt6o',
  ],
  ttl: 300,
});

new aws.route53.Record('typie.co|mx', {
  zoneId: zones.typie_co.zoneId,
  type: 'MX',
  name: 'typie.co',
  records: ['1 smtp.google.com'],
  ttl: 300,
});

new aws.route53.Record('help.typie.co', {
  zoneId: zones.typie_co.zoneId,
  type: 'CNAME',
  name: 'help.typie.co',
  // spell-checker:disable-next-line
  records: ['cname.rdbl.io'],
  ttl: 300,
});
