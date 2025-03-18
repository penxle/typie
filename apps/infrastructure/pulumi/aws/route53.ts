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
