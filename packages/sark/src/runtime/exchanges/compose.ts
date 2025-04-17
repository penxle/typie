import { share } from 'wonka';
import type { Exchange } from '../types';

export const composeExchanges = (exchanges: Exchange[]): Exchange => {
  return ({ forward }) => {
    return exchanges.reduceRight((forward, exchange) => {
      return exchange({
        forward: (ops$) => {
          return share(forward(share(ops$)));
        },
      });
    }, forward);
  };
};
