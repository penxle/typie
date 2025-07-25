import * as k8s from '@pulumi/kubernetes';
import { IAMServiceAccount } from '$components';
import { issuers } from './cert-manager';

const serviceAccount = new IAMServiceAccount('external-secrets', {
  metadata: {
    name: 'external-secrets',
    namespace: 'kube-system',
  },
  spec: {
    policy: {
      Version: '2012-10-17',
      Statement: [
        {
          Action: ['secretsmanager:ListSecrets', 'secretsmanager:BatchGetSecretValue'],
          Effect: 'Allow',
          Resource: '*',
        },
        {
          Effect: 'Allow',
          Action: [
            'secretsmanager:GetResourcePolicy',
            'secretsmanager:GetSecretValue',
            'secretsmanager:DescribeSecret',
            'secretsmanager:ListSecretVersionIds',
          ],
          Resource: ['*'],
        },
      ],
    },
  },
});

new k8s.helm.v4.Chart('external-secrets', {
  chart: 'external-secrets',
  namespace: 'kube-system',
  repositoryOpts: {
    repo: 'https://charts.external-secrets.io',
  },

  values: {
    serviceAccount: {
      create: false,
      name: serviceAccount.metadata.name,
    },

    webhook: {
      certManager: {
        enabled: true,
        cert: {
          issuerRef: {
            kind: issuers.selfSigned.kind,
            name: issuers.selfSigned.metadata.name,
          },
        },
      },
    },
  },
});

new k8s.apiextensions.CustomResource('ssm', {
  apiVersion: 'external-secrets.io/v1',
  kind: 'ClusterSecretStore',

  metadata: {
    name: 'ssm',
  },

  spec: {
    provider: {
      aws: {
        service: 'ParameterStore',
        region: 'ap-northeast-2',
      },
    },
  },
});

new k8s.apiextensions.CustomResource('secrets-manager', {
  apiVersion: 'external-secrets.io/v1',
  kind: 'ClusterSecretStore',

  metadata: {
    name: 'secrets-manager',
  },

  spec: {
    provider: {
      aws: {
        service: 'SecretsManager',
        region: 'ap-northeast-2',
      },
    },
  },
});
