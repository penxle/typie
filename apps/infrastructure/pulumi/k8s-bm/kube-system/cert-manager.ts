import * as k8s from '@pulumi/kubernetes';
import { IAMUserSecret } from '$components';
import { provider } from '$k8s-bm/provider';

const secret = new IAMUserSecret(
  'cert-manager',
  {
    metadata: {
      name: 'cert-manager',
      namespace: 'kube-system',
    },
    spec: {
      policy: {
        Version: '2012-10-17',
        Statement: [
          {
            Effect: 'Allow',
            Action: 'route53:GetChange',
            Resource: 'arn:aws:route53:::change/*',
          },
          {
            Effect: 'Allow',
            Action: ['route53:ChangeResourceRecordSets', 'route53:ListResourceRecordSets'],
            Resource: 'arn:aws:route53:::hostedzone/*',
          },
          {
            Effect: 'Allow',
            Action: 'route53:ListHostedZonesByName',
            Resource: '*',
          },
        ],
      },
    },
  },
  { provider },
);

const chart = new k8s.helm.v4.Chart(
  'cert-manager@bm',
  {
    name: 'cert-manager',

    chart: 'cert-manager',
    namespace: 'kube-system',
    repositoryOpts: {
      repo: 'https://charts.jetstack.io',
    },

    values: {
      crds: {
        enabled: true,
      },

      extraEnv: [
        { name: 'AWS_REGION', valueFrom: { secretKeyRef: { name: secret.metadata.name, key: 'AWS_REGION' } } },
        { name: 'AWS_ACCESS_KEY_ID', valueFrom: { secretKeyRef: { name: secret.metadata.name, key: 'AWS_ACCESS_KEY_ID' } } },
        { name: 'AWS_SECRET_ACCESS_KEY', valueFrom: { secretKeyRef: { name: secret.metadata.name, key: 'AWS_SECRET_ACCESS_KEY' } } },
      ],
    },
  },
  { provider },
);

const selfSignedIssuer = new k8s.apiextensions.CustomResource(
  'self-signed@bm',
  {
    apiVersion: 'cert-manager.io/v1',
    kind: 'ClusterIssuer',

    metadata: {
      name: 'self-signed',
    },

    spec: {
      selfSigned: {},
    },
  },
  { provider, dependsOn: chart },
);

const letsencryptStagingIssuer = new k8s.apiextensions.CustomResource(
  'letsencrypt-staging@bm',
  {
    apiVersion: 'cert-manager.io/v1',
    kind: 'ClusterIssuer',

    metadata: {
      name: 'letsencrypt-staging',
    },

    spec: {
      acme: {
        server: 'https://acme-staging-v02.api.letsencrypt.org/directory',
        email: 'cert@penxle.io',
        privateKeySecretRef: {
          name: 'letsencrypt-staging',
        },
        solvers: [{ dns01: { route53: {} } }],
      },
    },
  },
  { provider, dependsOn: chart },
);

const letsencryptIssuer = new k8s.apiextensions.CustomResource(
  'letsencrypt@bm',
  {
    apiVersion: 'cert-manager.io/v1',
    kind: 'ClusterIssuer',

    metadata: {
      name: 'letsencrypt',
    },

    spec: {
      acme: {
        server: 'https://acme-v02.api.letsencrypt.org/directory',
        email: 'cert@penxle.io',
        privateKeySecretRef: {
          name: 'letsencrypt',
        },
        solvers: [{ dns01: { route53: {} } }],
      },
    },
  },
  { provider, dependsOn: chart },
);

export const issuers = {
  selfSigned: selfSignedIssuer,
  letsencryptStaging: letsencryptStagingIssuer,
  letsencrypt: letsencryptIssuer,
};
