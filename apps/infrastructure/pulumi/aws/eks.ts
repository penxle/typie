import * as aws from '@pulumi/aws';
import { roles } from '$aws/iam';
import { securityGroups, subnets } from '$aws/vpc';

const clusterRole = new aws.iam.Role('cluster@eks', {
  name: 'cluster@eks',
  assumeRolePolicy: aws.iam.assumeRolePolicyForPrincipal({
    Service: 'eks.amazonaws.com',
  }),
  managedPolicyArns: [aws.iam.ManagedPolicy.AmazonEKSClusterPolicy],
});

export const cluster = new aws.eks.Cluster('typie', {
  name: 'typie',
  version: '1.33',
  roleArn: clusterRole.arn,

  bootstrapSelfManagedAddons: false,

  upgradePolicy: {
    supportType: 'STANDARD',
  },

  accessConfig: {
    authenticationMode: 'API',
  },

  vpcConfig: {
    subnetIds: [subnets.public.az1.id, subnets.public.az2.id, subnets.private.az1.id, subnets.private.az2.id],
    securityGroupIds: [securityGroups.internal.id],
    endpointPublicAccess: false,
    endpointPrivateAccess: true,
  },

  kubernetesNetworkConfig: {
    serviceIpv4Cidr: '10.100.0.0/16',
  },
});

export const nodeRole = new aws.iam.Role('node@eks', {
  name: 'node@eks',
  assumeRolePolicy: aws.iam.assumeRolePolicyForPrincipal({
    Service: 'ec2.amazonaws.com',
  }),
  managedPolicyArns: [aws.iam.ManagedPolicy.AmazonEKSWorkerNodePolicy, aws.iam.ManagedPolicy.AmazonEC2ContainerRegistryReadOnly],
});

new aws.eks.AccessEntry('node@eks', {
  clusterName: cluster.name,
  principalArn: nodeRole.arn,
  type: 'EC2_LINUX',
});

const launchTemplate = new aws.ec2.LaunchTemplate('eks-system', {
  name: 'eks-system',
  instanceType: 't4g.medium',
  vpcSecurityGroupIds: [securityGroups.internal.id],
  tagSpecifications: [
    {
      resourceType: 'instance',
      tags: {
        Name: 'system@eks',
      },
    },
  ],
});

new aws.eks.NodeGroup('system', {
  clusterName: cluster.name,
  nodeGroupName: 'system',

  nodeRoleArn: nodeRole.arn,

  capacityType: 'ON_DEMAND',
  scalingConfig: {
    minSize: 2,
    desiredSize: 2,
    maxSize: 2,
  },

  amiType: 'AL2023_ARM_64_STANDARD',
  subnetIds: [subnets.private.az1.id, subnets.private.az2.id],

  launchTemplate: {
    id: launchTemplate.id,
    version: launchTemplate.latestVersion.apply((v) => v.toString()),
  },

  taints: [{ key: 'CriticalAddonsOnly', value: 'true', effect: 'NO_SCHEDULE' }],
});

const actions = new aws.eks.AccessEntry('actions@github', {
  clusterName: cluster.name,
  principalArn: roles.actions.arn,
});

new aws.eks.AccessPolicyAssociation(
  'actions@github',
  {
    clusterName: cluster.name,
    principalArn: roles.actions.arn,
    policyArn: 'arn:aws:eks::aws:cluster-access-policy/AmazonEKSClusterAdminPolicy',
    accessScope: { type: 'cluster' },
  },
  { dependsOn: [actions] },
);

const vpcCniRole = new aws.iam.Role('vpc-cni@eks', {
  name: 'vpc-cni@eks',
  assumeRolePolicy: {
    Version: '2012-10-17',
    Statement: [
      {
        Effect: 'Allow',
        Principal: { Service: 'pods.eks.amazonaws.com' },
        Action: ['sts:AssumeRole', 'sts:TagSession'],
      },
    ],
  },
  managedPolicyArns: [aws.iam.ManagedPolicy.AmazonEKS_CNI_Policy],
});

const awsEbsCsiDriverRole = new aws.iam.Role('aws-ebs-csi-driver@eks', {
  name: 'aws-ebs-csi-driver@eks',
  assumeRolePolicy: {
    Version: '2012-10-17',
    Statement: [
      {
        Effect: 'Allow',
        Principal: { Service: 'pods.eks.amazonaws.com' },
        Action: ['sts:AssumeRole', 'sts:TagSession'],
      },
    ],
  },
  // spell-checker:disable-next-line
  managedPolicyArns: [aws.iam.ManagedPolicy.AmazonEBSCSIDriverPolicy],
});

new aws.eks.Addon('eks-pod-identity-agent', {
  clusterName: cluster.name,
  addonName: 'eks-pod-identity-agent',
  addonVersion: aws.eks.getAddonVersionOutput({
    addonName: 'eks-pod-identity-agent',
    kubernetesVersion: cluster.version,
    mostRecent: true,
  }).version,
});

new aws.eks.Addon('coredns', {
  clusterName: cluster.name,
  addonName: 'coredns',
  addonVersion: aws.eks.getAddonVersionOutput({
    addonName: 'coredns',
    kubernetesVersion: cluster.version,
    mostRecent: true,
  }).version,
});

new aws.eks.Addon('kube-proxy', {
  clusterName: cluster.name,
  addonName: 'kube-proxy',
  addonVersion: aws.eks.getAddonVersionOutput({
    addonName: 'kube-proxy',
    kubernetesVersion: cluster.version,
    mostRecent: true,
  }).version,
});

new aws.eks.Addon('vpc-cni', {
  clusterName: cluster.name,
  addonName: 'vpc-cni',
  addonVersion: aws.eks.getAddonVersionOutput({
    addonName: 'vpc-cni',
    kubernetesVersion: cluster.version,
    mostRecent: true,
  }).version,
  podIdentityAssociations: [{ roleArn: vpcCniRole.arn, serviceAccount: 'aws-node' }],
});

new aws.eks.Addon('aws-ebs-csi-driver', {
  clusterName: cluster.name,
  addonName: 'aws-ebs-csi-driver',
  addonVersion: aws.eks.getAddonVersionOutput({
    addonName: 'aws-ebs-csi-driver',
    kubernetesVersion: cluster.version,
    mostRecent: true,
  }).version,
  podIdentityAssociations: [{ roleArn: awsEbsCsiDriverRole.arn, serviceAccount: 'ebs-csi-controller-sa' }],
  configurationValues: JSON.stringify({
    controller: {
      volumeModificationFeature: {
        enabled: true,
      },
    },
  }),
});
