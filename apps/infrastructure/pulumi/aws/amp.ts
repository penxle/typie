import * as aws from '@pulumi/aws';

export const workspace = new aws.amp.Workspace('prometheus', {
  alias: 'prometheus',
});
