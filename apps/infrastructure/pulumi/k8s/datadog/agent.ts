import * as k8s from '@pulumi/kubernetes';
import { namespace } from './namespace';

new k8s.apiextensions.CustomResource('datadog-agent', {
  apiVersion: 'datadoghq.com/v2alpha1',
  kind: 'DatadogAgent',

  metadata: {
    name: 'datadog-agent',
    namespace: namespace.metadata.name,
  },

  spec: {
    global: {
      site: 'ap1.datadoghq.com',
      credentials: {
        apiSecret: {
          secretName: 'datadog-keys',
          keyName: 'api-key',
        },
        appSecret: {
          secretName: 'datadog-keys',
          keyName: 'app-key',
        },
      },

      clusterName: 'eks',
    },

    features: {
      clusterChecks: {
        useClusterChecksRunners: true,
      },
    },

    override: {
      clusterAgent: {
        replicas: 2,
      },
    },
  },
});
