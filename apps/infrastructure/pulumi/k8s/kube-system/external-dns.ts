import * as k8s from '@pulumi/kubernetes';
import { tables } from '$aws/dynamodb';
import { IAMServiceAccount } from '$components';

const serviceAccount = new IAMServiceAccount('external-dns', {
  metadata: {
    name: 'external-dns',
    namespace: 'kube-system',
  },
  spec: {
    policy: {
      Version: '2012-10-17',
      Statement: [
        {
          Effect: 'Allow',
          Action: ['route53:ChangeResourceRecordSets'],
          Resource: ['arn:aws:route53:::hostedzone/*'],
        },
        {
          Effect: 'Allow',
          Action: ['route53:ListHostedZones', 'route53:ListResourceRecordSets', 'route53:ListTagsForResource'],
          Resource: ['*'],
        },
        {
          Effect: 'Allow',
          Action: ['DynamoDB:DescribeTable', 'DynamoDB:PartiQLDelete', 'DynamoDB:PartiQLInsert', 'DynamoDB:PartiQLUpdate', 'DynamoDB:Scan'],
          Resource: [tables.externalDns.arn],
        },
      ],
    },
  },
});

new k8s.helm.v4.Chart('external-dns', {
  chart: 'external-dns',
  namespace: 'kube-system',
  repositoryOpts: {
    repo: 'https://kubernetes-sigs.github.io/external-dns',
  },

  values: {
    provider: 'aws',
    policy: 'sync',

    interval: '1m',
    triggerLoopOnEvent: true,
    sources: ['ingress', 'service'],

    registry: 'dynamodb',
    txtOwnerId: 'eks',

    extraArgs: ['--aws-zones-cache-duration=1h'],

    serviceAccount: {
      create: false,
      name: serviceAccount.metadata.name,
    },
  },
});
