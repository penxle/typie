import * as aws from '@pulumi/aws';
import { zones } from '$aws/route53';

new aws.route53.Record('talos.k8s.typie.io', {
  zoneId: zones.typie_io.zoneId,
  type: 'A',
  name: 'talos.k8s.typie.io',
  records: ['10.0.10.3'],
  ttl: 300,
});

new aws.route53.Record('controlplane.k8s.typie.io', {
  zoneId: zones.typie_io.zoneId,
  type: 'A',
  name: 'controlplane.k8s.typie.io',
  records: ['115.68.42.145'],
  ttl: 300,
});

new aws.route53.Record('ingress.k8s.typie.io', {
  zoneId: zones.typie_io.zoneId,
  type: 'A',
  name: 'ingress.k8s.typie.io',
  records: ['115.68.42.155'],
  ttl: 300,
});
