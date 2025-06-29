import * as aws from '@pulumi/aws';
import * as random from '@pulumi/random';
import { securityGroups, subnets } from '$aws/vpc';

const password = new random.RandomPassword('typie@rds', {
  length: 20,
  special: false,
});

const devPassword = new random.RandomPassword('typie-dev@rds', {
  length: 20,
  special: false,
});

const subnetGroup = new aws.rds.SubnetGroup('private', {
  name: 'private',
  description: 'Private subnets',
  subnetIds: [subnets.private.az1.id, subnets.private.az2.id],
});

const monitoringRole = new aws.iam.Role('monitoring@rds', {
  name: 'monitoring@rds',
  assumeRolePolicy: aws.iam.assumeRolePolicyForPrincipal({
    Service: 'monitoring.rds.amazonaws.com',
  }),
  managedPolicyArns: [aws.iam.ManagedPolicy.AmazonRDSEnhancedMonitoringRole],
});

const parameterGroup = new aws.rds.ClusterParameterGroup('typie-aurora-postgresql17', {
  name: 'typie-aurora-postgresql17',
  family: 'aurora-postgresql17',

  parameters: [
    { name: 'default_toast_compression', value: 'lz4' },
    { name: 'pg_stat_statements.track', value: 'ALL' },
    { name: 'pg_stat_statements.max', value: '10000', applyMethod: 'pending-reboot' },
    { name: 'pg_stat_statements.track_utility', value: '0' },
  ],
});

const cluster = new aws.rds.Cluster('typie', {
  clusterIdentifier: 'typie',

  engine: 'aurora-postgresql',
  engineMode: 'provisioned',
  engineVersion: '17.4',

  // spell-checker:disable-next-line
  storageType: 'aurora-iopt1',

  dbClusterParameterGroupName: parameterGroup.name,

  dbSubnetGroupName: subnetGroup.name,
  vpcSecurityGroupIds: [securityGroups.internal.id],

  deletionProtection: true,
  storageEncrypted: true,

  backupRetentionPeriod: 7,
  finalSnapshotIdentifier: 'typie-final-snapshot',

  monitoringInterval: 60,
  monitoringRoleArn: monitoringRole.arn,

  preferredBackupWindow: '19:00-20:00',
  preferredMaintenanceWindow: 'sun:20:00-sun:22:00',

  masterUsername: 'root',
  masterPassword: password.result,

  allowMajorVersionUpgrade: true,
  applyImmediately: true,
});

const instance = new aws.rds.ClusterInstance('typie-1', {
  clusterIdentifier: cluster.id,
  identifier: 'typie-1',

  engine: 'aurora-postgresql',
  instanceClass: 'db.r7g.large',

  availabilityZone: subnets.private.az1.availabilityZone,
  caCertIdentifier: 'rds-ca-ecc384-g1',

  monitoringInterval: 60,

  preferredMaintenanceWindow: 'sun:20:00-sun:22:00',

  promotionTier: 0,

  applyImmediately: true,
});

const devCluster = new aws.rds.Cluster('typie-dev', {
  clusterIdentifier: 'typie-dev',

  engine: 'aurora-postgresql',
  engineMode: 'provisioned',
  engineVersion: '17.4',

  dbClusterParameterGroupName: parameterGroup.name,

  dbSubnetGroupName: subnetGroup.name,
  vpcSecurityGroupIds: [securityGroups.internal.id],

  deletionProtection: true,
  storageEncrypted: true,

  backupRetentionPeriod: 7,
  finalSnapshotIdentifier: 'typie-dev-final-snapshot',

  preferredBackupWindow: '19:00-20:00',
  preferredMaintenanceWindow: 'sun:20:00-sun:22:00',

  masterUsername: 'root',
  masterPassword: devPassword.result,

  allowMajorVersionUpgrade: true,
  applyImmediately: true,
});

new aws.rds.ClusterInstance('typie-dev-1', {
  clusterIdentifier: devCluster.id,
  identifier: 'typie-dev-1',

  engine: 'aurora-postgresql',
  instanceClass: 'db.t4g.medium',

  availabilityZone: subnets.private.az1.availabilityZone,
  caCertIdentifier: 'rds-ca-ecc384-g1',

  preferredMaintenanceWindow: 'sun:20:00-sun:22:00',

  promotionTier: 0,

  applyImmediately: true,
});

export const db = {
  cluster,
  instance,
};

export const outputs = {
  AWS_RDS_PASSWORD: password.result,
  AWS_RDS_DEV_PASSWORD: devPassword.result,
};
