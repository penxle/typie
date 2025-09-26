import * as k8s from '@pulumi/kubernetes';
import { provider } from '$k8s-bm/provider';
import { namespace } from './namespace';

new k8s.helm.v4.Chart(
  'cloudnative-pg@bm',
  {
    name: 'cloudnative-pg',

    chart: 'cloudnative-pg',
    namespace: namespace.metadata.name,
    repositoryOpts: {
      repo: 'https://cloudnative-pg.github.io/charts',
    },
  },
  { provider },
);
