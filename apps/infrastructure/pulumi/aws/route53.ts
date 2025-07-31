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
    // spell-checker:disable
    'google-site-verification=Q-1ETLmF6p7XkzQM0wpDyF0wCBQREsjK1aZdxR-4ggQ',
    'google-site-verification=hZdtWP44my1tA-wUAvYlOKAAPSp2vHT6M5omQXCRt6o',
    'facebook-domain-verification=fduiqboyntm5jz4x19bf0pau0ii960',
    // spell-checker:enable
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

export const outputs = {
  AWS_ROUTE53_TYPIE_CO_ZONE_ID: zones.typie_co.zoneId,
  AWS_ROUTE53_TYPIE_DEV_ZONE_ID: zones.typie_dev.zoneId,
  AWS_ROUTE53_TYPIE_ME_ZONE_ID: zones.typie_me.zoneId,
  AWS_ROUTE53_TYPIE_NET_ZONE_ID: zones.typie_net.zoneId,
  AWS_ROUTE53_TYPIE_IO_ZONE_ID: zones.typie_io.zoneId,
};
