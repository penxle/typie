import { PortOneClient } from '@portone/server-sdk';
import { match, P } from 'ts-pattern';
import { env } from '../env';

export const client = PortOneClient({ secret: env.PORTONE_API_SECRET });

type PortOneSuccessResult<T> = { status: 'succeeded' } & T;
type PortOneFailureResult = { status: 'failed'; code: string; message: string };

type PortOneResult<T> = PortOneSuccessResult<T> | PortOneFailureResult;

type GetPaymentParams = {
  paymentId: string;
};
type GetPaymentResult = PortOneResult<{ amount: number }>;
export const getPayment = async (params: GetPaymentParams): Promise<GetPaymentResult> => {
  const resp = await client.payment.getPayment({
    paymentId: params.paymentId,
  });

  if (resp.status === 'PAID') {
    return makeSuccessResult(resp);
  }

  return makeFailureResult(resp);
};

const makeSuccessResult = <T>(data: T): PortOneSuccessResult<T> => {
  return { status: 'succeeded', ...data };
};

const makeFailureResult = (error: unknown): PortOneFailureResult => {
  // TODO: 에러 구분이 빠졌는데 나중에 필요
  return {
    status: 'failed',
    ...match(error)
      .with(P.instanceOf(Error), (e) => ({ code: e.message, message: e.message }))
      .otherwise((e) => ({ code: 'unknown', message: String(e) })),
  };
};
