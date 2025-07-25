import * as k8s from '@pulumi/kubernetes';
import { IAMServiceAccount } from '$components';

const serviceAccount = new IAMServiceAccount('cert-manager', {
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
});

new k8s.helm.v4.Chart('cert-manager', {
  chart: 'cert-manager',
  namespace: 'kube-system',
  repositoryOpts: {
    repo: 'https://charts.jetstack.io',
  },

  values: {
    crds: {
      enabled: true,
    },

    serviceAccount: {
      create: false,
      name: serviceAccount.metadata.name,
    },
  },
});

const selfSignedIssuer = new k8s.apiextensions.CustomResource('self-signed', {
  apiVersion: 'cert-manager.io/v1',
  kind: 'ClusterIssuer',

  metadata: {
    name: 'self-signed',
  },

  spec: {
    selfSigned: {},
  },
});

const letsencryptStagingIssuer = new k8s.apiextensions.CustomResource('letsencrypt-staging', {
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
});

const letsencryptIssuer = new k8s.apiextensions.CustomResource('letsencrypt', {
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
});

export const issuers = {
  selfSigned: selfSignedIssuer,
  letsencryptStaging: letsencryptStagingIssuer,
  letsencrypt: letsencryptIssuer,
};
