import * as k8s from '@pulumi/kubernetes';
import { provider } from '$k8s-bm/provider';

export const namespace = new k8s.core.v1.Namespace(
  'datadog@bm',
  {
    metadata: { name: 'datadog' },
  },
  { provider },
);
