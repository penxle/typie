import { logger } from '@typie/lib';
import { Connection } from 'rabbitmq-client';
import { env } from '@/env';

const log = logger.getChild('mq');

export const rabbit = new Connection(env.RABBITMQ_URL);

rabbit.on('error', (error) => {
  log.error('RabbitMQ connection error {*}', { error });
});
