import { z } from 'zod';

export const userSchema = {
  name: z.string().trim().min(1, '이름을 입력해주세요').max(20, '이름은 20자를 넘을 수 없어요'),
};

export const cardSchema = {
  cardNumber: z
    .string({ required_error: '카드 번호를 입력해 주세요' })
    .transform((str) => str.replaceAll(/\D/g, ''))
    .refine((str) => str.length >= 15 && str.length <= 16, { message: '올바른 카드 번호를 입력해 주세요' }),

  expiryDate: z
    .string({ required_error: '만료일을 입력해 주세요' })
    .transform((str) => str.replaceAll(/\D/g, ''))
    .refine((str) => str.length === 4, { message: '올바른 만료일을 입력해 주세요' }),

  birthOrBusinessRegistrationNumber: z
    .string({ required_error: '생년월일 또는 사업자 등록번호를 입력해 주세요' })
    .transform((str) => str.replaceAll(/\D/g, ''))
    .transform((str) => (str.length === 8 ? str.slice(2) : str)) // YYYYMMDD -> YYMMDD
    .refine((str) => str.length === 6 || str.length === 10, {
      message: '올바른 생년월일 또는 사업자 등록번호를 입력해 주세요',
    }),

  passwordTwoDigits: z
    .string({ required_error: '카드 비밀번호를 입력해 주세요' })
    .regex(/^\d{2}$/, { message: '올바른 카드 비밀번호를 입력해 주세요' }),
};
