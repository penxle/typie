import * as k8s from '@pulumi/kubernetes';
import { namespace } from './namespace';

new k8s.helm.v4.Chart('cloudnative-pg', {
  chart: 'cloudnative-pg',
  namespace: namespace.metadata.name,
  repositoryOpts: {
    repo: 'https://cloudnative-pg.github.io/charts',
  },
  values: {
    replicaCount: 2,
    monitoring: {
      podMonitorEnabled: true,
      grafanaDashboard: {
        create: true,
      },
    },
  },
});
