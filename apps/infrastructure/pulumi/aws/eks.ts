import * as aws from '@pulumi/aws';
import * as tls from '@pulumi/tls';
import { roles } from '$aws/iam';
import { securityGroups, subnets } from '$aws/vpc';

const clusterRole = new aws.iam.Role('cluster@eks', {
  name: 'cluster@eks',
  assumeRolePolicy: aws.iam.assumeRolePolicyForPrincipal({
    Service: 'eks.amazonaws.com',
  }),
  managedPolicyArns: [aws.iam.ManagedPolicy.AmazonEKSClusterPolicy],
});

export const cluster = new aws.eks.Cluster('glitter', {
  name: 'glitter',
  version: '1.32',
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

const certificate = tls.getCertificateOutput({
  url: cluster.identities[0].oidcs[0].issuer,
});

export const oidcProvider = new aws.iam.OpenIdConnectProvider('cluster@eks', {
  url: cluster.identities[0].oidcs[0].issuer,
  clientIdLists: ['sts.amazonaws.com'],
  thumbprintLists: [certificate.certificates[0].sha1Fingerprint],
});

const admin = new aws.eks.AccessEntry('admin@team', {
  clusterName: cluster.name,
  principalArn: roles.admin.arn,
});

new aws.eks.AccessPolicyAssociation(
  'admin@team',
  {
    clusterName: cluster.name,
    principalArn: roles.admin.arn,
    policyArn: 'arn:aws:eks::aws:cluster-access-policy/AmazonEKSClusterAdminPolicy',
    accessScope: { type: 'cluster' },
  },
  { dependsOn: [admin] },
);

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
        Principal: { Federated: oidcProvider.arn },
        Action: 'sts:AssumeRoleWithWebIdentity',
        Condition: oidcProvider.url.apply((url) => ({
          StringEquals: {
            [`${url}:aud`]: 'sts.amazonaws.com',
            [`${url}:sub`]: 'system:serviceaccount:kube-system:aws-node',
          },
        })),
      },
    ],
  },
  // spell-checker:disable-next-line
  managedPolicyArns: [aws.iam.ManagedPolicy.AmazonEKS_CNI_Policy],
});

const awsEbsCsiDriverRole = new aws.iam.Role('aws-ebs-csi-driver@eks', {
  name: 'aws-ebs-csi-driver@eks',
  assumeRolePolicy: {
    Version: '2012-10-17',
    Statement: [
      {
        Effect: 'Allow',
        Principal: { Federated: oidcProvider.arn },
        Action: 'sts:AssumeRoleWithWebIdentity',
        Condition: oidcProvider.url.apply((url) => ({
          StringEquals: {
            [`${url}:aud`]: 'sts.amazonaws.com',
            [`${url}:sub`]: 'system:serviceaccount:kube-system:ebs-csi-controller-sa',
          },
        })),
      },
    ],
  },
  // spell-checker:disable-next-line
  managedPolicyArns: [aws.iam.ManagedPolicy.AmazonEBSCSIDriverPolicy],
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
  serviceAccountRoleArn: vpcCniRole.arn,
});

new aws.eks.Addon('aws-ebs-csi-driver', {
  clusterName: cluster.name,
  addonName: 'aws-ebs-csi-driver',
  addonVersion: aws.eks.getAddonVersionOutput({
    addonName: 'aws-ebs-csi-driver',
    kubernetesVersion: cluster.version,
    mostRecent: true,
  }).version,
  serviceAccountRoleArn: awsEbsCsiDriverRole.arn,
  configurationValues: JSON.stringify({
    controller: {
      volumeModificationFeature: {
        enabled: true,
      },
    },
  }),
});

export const outputs = {
  AWS_EKS_OIDC_PROVIDER_ARN: oidcProvider.arn,
  AWS_EKS_OIDC_PROVIDER_URL: oidcProvider.url,
};
