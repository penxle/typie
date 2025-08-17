import * as k8s from '@pulumi/kubernetes';
import * as pulumi from '@pulumi/pulumi';
import * as random from '@pulumi/random';
import { db } from '$aws/rds';
import { namespace } from './namespace';

const password = new random.RandomPassword('dbm@datadog', {
  length: 20,
  special: false,
});

new k8s.core.v1.Service('dbm@datadog', {
  metadata: {
    name: 'dbm',
    namespace: namespace.metadata.name,
    annotations: {
      'ad.datadoghq.com/service.checks': pulumi.jsonStringify({
        postgres: {
          init_config: {},
          instances: [
            {
              dbm: true,
              host: db.instance.endpoint,
              username: 'datadog',
              password: password.result,
              database_autodiscovery: { enabled: true },
              collect_schemas: { enabled: true },
              relations: [{ relation_regex: '.*' }],
            },
          ],
        },
      }),
    },
  },
  spec: {
    type: 'ClusterIP',
    clusterIP: 'None',
    ports: [{ port: 5432 }],
  },
});

export const outputs = {
  K8S_DATADOG_DBM_PASSWORD: password.result,
};
