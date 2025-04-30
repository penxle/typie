import * as k8s from '@pulumi/kubernetes';
import { namespace } from './index';

new k8s.helm.v4.Chart('datadog-operator', {
  chart: 'datadog-operator',
  namespace: namespace.metadata.name,
  repositoryOpts: {
    repo: 'https://helm.datadoghq.com',
  },
});
