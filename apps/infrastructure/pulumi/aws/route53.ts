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

new aws.route53.Record('glttr.io|txt', {
  zoneId: zones.glttr_io.zoneId,
  type: 'TXT',
  name: 'glttr.io',
  records: [
    // spell-checker:disable-next-line
    'google-site-verification=A464_1vmgr8nj1yaU_VjcAjJspVjB9sLHJloGKBSy7o',
  ],
  ttl: 300,
});
