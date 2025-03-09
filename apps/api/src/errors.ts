import { GraphQLError } from 'graphql';

type GlitterErrorParams = {
  code: string;
  message?: string;
  status?: number;
};

export class GlitterError extends GraphQLError {
  public code: string;
  public status: number;

  constructor({ code, message, status }: GlitterErrorParams) {
    super(message ?? code, { extensions: { type: 'GlitterError', code, status } });
    this.name = 'GlitterError';
    this.code = code;
    this.status = status ?? 500;
  }
}
