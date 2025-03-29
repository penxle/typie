import { GraphQLError } from 'graphql';

type TypieErrorParams = {
  code: string;
  message?: string;
  status?: number;
};

export class TypieError extends GraphQLError {
  public code: string;
  public status: number;

  constructor({ code, message, status }: TypieErrorParams) {
    super(message ?? code, { extensions: { type: 'TypieError', code, status } });
    this.name = 'TypieError';
    this.code = code;
    this.status = status ?? 500;
  }
}
