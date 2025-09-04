import * as k8s from '@pulumi/kubernetes';
import * as pulumi from '@pulumi/pulumi';
import { namespace } from './namespace';

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
              reported_hostname: 'db.typie.io',
              host: 'db-rw.prod.svc.cluster.local',
              username: 'datadog',
              // spell-checker:disable
              dbname: 'app',
              dbstrict: true,
              // spell-checker:enable
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
