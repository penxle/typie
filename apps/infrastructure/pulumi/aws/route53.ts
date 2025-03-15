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
