import * as aws from '@pulumi/aws';
import * as random from '@pulumi/random';
import { zones } from '$aws/route53';
import { securityGroups, subnets } from '$aws/vpc';

const password = new random.RandomPassword('glitter@rds', {
  length: 20,
  special: false,
});

const subnetGroup = new aws.rds.SubnetGroup('private', {
  name: 'private',
  description: 'Private subnets',
  subnetIds: [subnets.private.az1.id, subnets.private.az2.id],
});

const parameterGroup = new aws.rds.ClusterParameterGroup('glitter', {
  name: 'glitter-aurora-postgresql16',
  family: 'aurora-postgresql16',

  parameters: [
    { name: 'pg_stat_statements.track', value: 'ALL' },
    { name: 'pg_stat_statements.max', value: '10000', applyMethod: 'pending-reboot' },
    { name: 'pg_stat_statements.track_utility', value: '0' },
  ],
});

const cluster = new aws.rds.Cluster('glitter', {
  clusterIdentifier: 'glitter',

  engine: 'aurora-postgresql',
  engineMode: 'provisioned',
  engineVersion: '16.6',

  dbClusterParameterGroupName: parameterGroup.name,

  dbSubnetGroupName: subnetGroup.name,
  vpcSecurityGroupIds: [securityGroups.internal.id],

  deletionProtection: true,
  storageEncrypted: true,

  backupRetentionPeriod: 7,
  finalSnapshotIdentifier: 'glitter-final-snapshot',

  preferredBackupWindow: '19:00-20:00',
  preferredMaintenanceWindow: 'sun:20:00-sun:22:00',

  performanceInsightsEnabled: true,

  masterUsername: 'root',
  masterPassword: password.result,

  applyImmediately: true,
});

const instance = new aws.rds.ClusterInstance('glitter-1', {
  clusterIdentifier: cluster.id,
  identifier: 'glitter-1',

  engine: 'aurora-postgresql',
  instanceClass: 'db.t4g.medium',

  availabilityZone: subnets.private.az1.availabilityZone,
  caCertIdentifier: 'rds-ca-ecc384-g1',

  preferredMaintenanceWindow: 'sun:20:00-sun:22:00',

  promotionTier: 0,

  applyImmediately: true,
});

// new aws.rds.ClusterInstance('glitter-2', {
//   clusterIdentifier: cluster.id,
//   identifier: 'glitter-2',

//   engine: 'aurora-postgresql',
//   instanceClass: 'db.t4g.medium',

//   availabilityZone: subnets.private.az2.availabilityZone,
//   caCertIdentifier: 'rds-ca-ecc384-g1',

//   preferredMaintenanceWindow: 'sun:20:00-sun:22:00',

//   promotionTier: 1,

//   applyImmediately: true,
// });

new aws.route53.Record('db.glitter.rocks', {
  zoneId: zones.glitter_rocks.zoneId,
  type: 'CNAME',
  name: 'db.glitter.rocks',
  records: [cluster.endpoint],
  ttl: 300,
});

export const db = {
  cluster,
  instance,
};

export const outputs = {
  AWS_RDS_PASSWORD: password.result,
};
