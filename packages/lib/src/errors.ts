import { GraphQLError } from 'graphql';

type TypieErrorParams = {
  code: string;
  message?: string;
  status?: number;
  extra?: unknown;
};

export class TypieError extends GraphQLError {
  public code: string;
  public status: number;
  public extra?: unknown;

  constructor({ code, message, status, extra }: TypieErrorParams) {
    super(message ?? code, { extensions: { type: 'TypieError', code, status, extra } });
    this.name = 'TypieError';
    this.code = code;
    this.status = status ?? 500;
    this.extra = extra;
  }
}

export class NotFoundError extends TypieError {
  constructor() {
    super({ code: 'not_found', status: 404 });
  }
}
