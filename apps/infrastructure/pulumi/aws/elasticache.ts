import * as aws from '@pulumi/aws';
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

new aws.elasticache.ReplicationGroup('typie', {
  replicationGroupId: 'typie',
  description: 'Valkey cluster',

  engine: 'valkey',
  engineVersion: '8.0',
  parameterGroupName: parameterGroup.name,

  nodeType: 'cache.t4g.medium',

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
