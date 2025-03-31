import type { Exchange } from './types';

export const composeExchanges = (exchanges: Exchange[]): Exchange => {
  return ({ forward }) => {
    return exchanges.reduceRight((forward, exchange) => {
      return exchange({ forward });
    }, forward);
  };
};
