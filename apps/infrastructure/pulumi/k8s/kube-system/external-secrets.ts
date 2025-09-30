import * as k8s from '@pulumi/kubernetes';
import { issuers } from './cert-manager';

new k8s.helm.v4.Chart('external-secrets', {
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
});
