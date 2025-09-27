import * as k8s from '@pulumi/kubernetes';
import { IAMUserSecret } from '$components';
import { provider } from '$k8s-bm/provider';

const secret = new IAMUserSecret(
  'external-dns',
  {
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
        ],
      },
    },
  },
  { provider },
);

new k8s.helm.v4.Chart(
  'external-dns@bm',
  {
    name: 'external-dns',

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
      sources: ['ingress', 'service', 'gateway-httproute'],
      annotationFilter: 'external-dns.typie.io/enabled=true',

      registry: 'txt',
      txtOwnerId: 'k8s',
      txtPrefix: 'ed-',

      extraArgs: ['--aws-zones-cache-duration=1h'],

      env: [
        { name: 'AWS_REGION', valueFrom: { secretKeyRef: { name: secret.metadata.name, key: 'AWS_REGION' } } },
        { name: 'AWS_ACCESS_KEY_ID', valueFrom: { secretKeyRef: { name: secret.metadata.name, key: 'AWS_ACCESS_KEY_ID' } } },
        { name: 'AWS_SECRET_ACCESS_KEY', valueFrom: { secretKeyRef: { name: secret.metadata.name, key: 'AWS_SECRET_ACCESS_KEY' } } },
      ],
    },
  },
  { provider },
);
