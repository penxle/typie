import * as aws from '@pulumi/aws';
import { zones } from '$aws/route53';
import { securityGroups, subnets } from '$aws/vpc';

const subnetGroup = new aws.elasticache.SubnetGroup('private', {
  name: 'private',
  description: 'Private subnets',
  subnetIds: [subnets.private.az1.id, subnets.private.az2.id],
});

const parameterGroup = new aws.elasticache.ParameterGroup('typie-valkey8-cluster', {
  name: 'typie-valkey8-cluster',
  family: 'valkey8',

  parameters: [
    { name: 'cluster-enabled', value: 'yes' },
    { name: 'maxmemory-policy', value: 'noeviction' },
  ],
});

const cluster = new aws.elasticache.ReplicationGroup('typie', {
  replicationGroupId: 'typie',
  description: 'Valkey cluster',

  engine: 'valkey',
  engineVersion: '8.0',
  parameterGroupName: parameterGroup.name,

  nodeType: 'cache.r7g.large',

  clusterMode: 'enabled',
  numNodeGroups: 1,
  replicasPerNodeGroup: 1,

  subnetGroupName: subnetGroup.name,
  securityGroupIds: [securityGroups.internal.id],

  multiAzEnabled: true,
  automaticFailoverEnabled: true,

  atRestEncryptionEnabled: true,
  transitEncryptionEnabled: false,

  snapshotRetentionLimit: 7,
  finalSnapshotIdentifier: 'typie-final-snapshot',

  snapshotWindow: '19:00-20:00',
  maintenanceWindow: 'sun:20:00-sun:22:00',

  applyImmediately: true,
});

new aws.route53.Record('redis.typie.io', {
  zoneId: zones.typie_io.zoneId,
  type: 'CNAME',
  name: 'redis.typie.io',
  records: [cluster.configurationEndpointAddress],
  ttl: 300,
});
