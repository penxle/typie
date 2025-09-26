import * as k8s from '@pulumi/kubernetes';
import { provider } from '$k8s-bm/provider';

new k8s.core.v1.Namespace(
  'dev@bm',
  {
    metadata: {
      name: 'dev',
    },
  },
  { provider },
);

new k8s.core.v1.Namespace(
  'prod@bm',
  {
    metadata: {
      name: 'prod',
    },
  },
  { provider },
);
