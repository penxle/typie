import * as k8s from '@pulumi/kubernetes';
import { provider } from '$k8s-bm/provider';
import { namespace } from './namespace';

new k8s.apiextensions.CustomResource(
  'datadog-agent@bm',
  {
    apiVersion: 'datadoghq.com/v2alpha1',
    kind: 'DatadogAgent',
    metadata: {
      name: 'datadog',
      namespace: namespace.metadata.name,
    },
    spec: {
      global: {
        clusterName: 'k8s',
        site: 'ap1.datadoghq.com',

        credentials: {
          apiSecret: {
            secretName: 'datadog-secret',
            keyName: 'api-key',
          },
          appSecret: {
            secretName: 'datadog-secret',
            keyName: 'app-key',
          },
        },

        kubelet: {
          tlsVerify: false,
        },
      },

      features: {
        processDiscovery: {
          enabled: false,
        },
      },

      override: {
        nodeAgent: {
          tolerations: [{ operator: 'Exists' }],
        },
      },
    },
  },
  { provider },
);
