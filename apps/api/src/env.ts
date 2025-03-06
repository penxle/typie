import { z } from 'zod';

const schema = z.object({
  LISTEN_PORT: z.coerce.number().optional(),
});

export const env = schema.parse(process.env);
export const dev = process.env.NODE_ENV !== 'production';
export const production = process.env.PUBLIC_PULUMI_STACK
  ? process.env.PUBLIC_PULUMI_STACK === 'prod'
  : process.env.DOPPLER_ENVIRONMENT === 'prod';
