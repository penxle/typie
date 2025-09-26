import * as k8s from '@pulumi/kubernetes';
import { provider } from '$k8s-bm/provider';
import { issuers } from './cert-manager';

new k8s.helm.v4.Chart(
  'external-secrets@bm',
  {
    name: 'external-secrets',

    chart: 'external-secrets',
    namespace: 'kube-system',
    repositoryOpts: {
      repo: 'https://charts.external-secrets.io',
    },

    values: {
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
  },
  { provider },
);
