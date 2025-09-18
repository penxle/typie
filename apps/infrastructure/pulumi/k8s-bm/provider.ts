import * as k8s from '@pulumi/kubernetes';

export const provider = new k8s.Provider('bm', {
  kubeconfig: '~/.kube/config',
});
