import * as k8s from '@pulumi/kubernetes';
import { namespace } from './namespace';

new k8s.helm.v4.Chart('rabbitmq-cluster-operator', {
  chart: 'rabbitmq-cluster-operator',
  namespace: namespace.metadata.name,
  repositoryOpts: {
    repo: 'https://charts.bitnami.com/bitnami',
  },
  values: {
    useCertManager: true,
  },
});
