import * as aws from '@pulumi/aws';
import * as k8s from '@pulumi/kubernetes';
import { cluster, nodeRole } from '$aws/eks';
import { securityGroups, subnets } from '$aws/vpc';
import { IAMServiceAccount } from '$components';

const serviceAccount = new IAMServiceAccount('karpenter', {
  metadata: {
    name: 'karpenter',
    namespace: 'kube-system',
  },
  spec: {
    policy: {
      Version: '2012-10-17',
      Statement: [
        {
          Action: [
            'ssm:GetParameter',
            'ec2:DescribeImages',
            'ec2:RunInstances',
            'ec2:DescribeSubnets',
            'ec2:DescribeSecurityGroups',
            'ec2:DescribeLaunchTemplates',
            'ec2:DescribeInstances',
            'ec2:DescribeInstanceTypes',
            'ec2:DescribeInstanceTypeOfferings',
            'ec2:DescribeAvailabilityZones',
            'ec2:DeleteLaunchTemplate',
            'ec2:CreateTags',
            'ec2:CreateLaunchTemplate',
            'ec2:CreateFleet',
            'ec2:DescribeSpotPriceHistory',
            'pricing:GetProducts',
          ],
          Effect: 'Allow',
          Resource: '*',
        },
        {
          Action: 'ec2:TerminateInstances',
          Effect: 'Allow',
          Resource: '*',
        },
        {
          Effect: 'Allow',
          Action: 'iam:PassRole',
          Resource: '*',
        },
        {
          Effect: 'Allow',
          Action: 'eks:DescribeCluster',
          Resource: '*',
        },
        {
          Effect: 'Allow',
          Action: ['iam:CreateInstanceProfile'],
          Resource: '*',
        },
        {
          Effect: 'Allow',
          Action: ['iam:TagInstanceProfile'],
          Resource: '*',
        },
        {
          Effect: 'Allow',
          Action: ['iam:AddRoleToInstanceProfile', 'iam:RemoveRoleFromInstanceProfile', 'iam:DeleteInstanceProfile'],
          Resource: '*',
        },
        {
          Effect: 'Allow',
          Action: 'iam:GetInstanceProfile',
          Resource: '*',
        },
        {
          Effect: 'Allow',
          Action: ['sqs:DeleteMessage', 'sqs:GetQueueUrl', 'sqs:ReceiveMessage'],
          Resource: '*',
        },
      ],
    },
  },
});

const interruptionQueue = new aws.sqs.Queue('karpenter-interruption', {
  name: 'karpenter-interruption',
  messageRetentionSeconds: 300,
  sqsManagedSseEnabled: true,
});

new aws.sqs.QueuePolicy('karpenter-interruption', {
  queueUrl: interruptionQueue.url,
  policy: {
    Version: '2012-10-17',
    Statement: [
      {
        Effect: 'Allow',
        Action: 'sqs:SendMessage',
        Resource: interruptionQueue.arn,
        Principal: {
          Service: ['events.amazonaws.com', 'sqs.amazonaws.com'],
        },
      },
    ],
  },
});

const declareEventRule = (name: string, source: string, detailType: string) => {
  const rule = new aws.cloudwatch.EventRule(name, {
    name,
    eventPattern: JSON.stringify({
      source: [source],
      'detail-type': [detailType],
    }),
  });

  new aws.cloudwatch.EventTarget(name, {
    targetId: name,
    rule: rule.name,
    arn: interruptionQueue.arn,
  });
};

declareEventRule('karpenter-interruption.scheduled-change', 'aws.health', 'AWS Health Event');
declareEventRule('karpenter-interruption.spot-interruption', 'aws.ec2', 'EC2 Spot Instance Interruption Warning');
declareEventRule('karpenter-interruption.rebalance', 'aws.ec2', 'EC2 Instance Rebalance Recommendation');
declareEventRule('karpenter-interruption.instance-state-change', 'aws.ec2', 'EC2 Instance State-change Notification');

const chart = new k8s.helm.v4.Chart('karpenter', {
  chart: 'oci://public.ecr.aws/karpenter/karpenter',
  namespace: 'kube-system',

  values: {
    serviceAccount: {
      create: false,
      name: serviceAccount.metadata.name,
    },

    podLabels: { app: 'karpenter' },

    settings: {
      clusterName: cluster.name,
      interruptionQueue: interruptionQueue.name,
      featureGates: {
        spotToSpotConsolidation: true,
      },
    },
  },
});

const nodeClass = new k8s.apiextensions.CustomResource(
  'node',
  {
    apiVersion: 'karpenter.k8s.aws/v1',
    kind: 'EC2NodeClass',

    metadata: {
      name: 'node',
    },

    spec: {
      role: nodeRole.name,

      amiSelectorTerms: [{ alias: 'al2023@latest' }],
      subnetSelectorTerms: [{ id: subnets.private.az1.id }, { id: subnets.private.az2.id }],
      securityGroupSelectorTerms: [{ id: securityGroups.internal.id }],

      blockDeviceMappings: [
        {
          // spell-checker:disable-next-line
          deviceName: '/dev/xvda',
          ebs: {
            volumeSize: '20Gi',
            volumeType: 'gp3',
            iops: 3000,
            throughput: 125,
          },
        },
      ],

      tags: {
        Name: 'node@eks',
      },
    },
  },
  { dependsOn: [chart] },
);

new k8s.apiextensions.CustomResource(
  'workload',
  {
    apiVersion: 'karpenter.sh/v1',
    kind: 'NodePool',

    metadata: {
      name: 'workload',
    },

    spec: {
      template: {
        spec: {
          nodeClassRef: {
            group: 'karpenter.k8s.aws',
            kind: nodeClass.kind,
            name: nodeClass.metadata.name,
          },

          requirements: [
            { key: 'kubernetes.io/arch', operator: 'In', values: ['arm64'] },
            { key: 'kubernetes.io/os', operator: 'In', values: ['linux'] },
            { key: 'karpenter.sh/capacity-type', operator: 'In', values: ['spot'] },
            { key: 'karpenter.k8s.aws/instance-category', operator: 'In', values: ['c', 'm', 'r'] },
            { key: 'karpenter.k8s.aws/instance-generation', operator: 'Gt', values: ['5'] },
          ],

          expireAfter: '720h', // 30 * 24h
        },
      },

      limits: {
        cpu: 1_000_000,
        memory: '1000000Gi',
      },

      disruption: {
        consolidationPolicy: 'WhenEmptyOrUnderutilized',
        consolidateAfter: '5m',
      },
    },
  },
  { dependsOn: [chart] },
);
