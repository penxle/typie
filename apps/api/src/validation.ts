import { z } from 'zod';

const UNAVAILABLE_SITE_SLUGS = {
  EXACT: ['admin', 'app', 'cname', 'dev', 'docs', 'help', 'template', 'www'],
};

export const userSchema = {
  name: z.string().trim().min(1, '이름을 입력해주세요').max(20, '이름은 20자를 넘을 수 없어요'),
};

export const siteSchema = {
  slug: z
    .string({ error: '사이트 주소를 입력해 주세요' })
    .trim()
    .toLowerCase()
    .min(4, { message: '사이트 주소는 4글자 이상이여야 해요' })
    .max(63, { message: '사이트 주소는 63글자를 넘을 수 없어요' })
    .regex(/^[\da-z-]+$/, { message: '사이트 주소는 소문자, 숫자, 하이픈만 사용할 수 있어요' })
    .regex(/^[\da-z][\da-z-]*[\da-z]$/, { message: '사이트 주소는 하이픈으로 시작하거나 끝날 수 없어요' })
    .refine((str) => !str.includes('--'), { message: '하이픈을 연속으로 사용할 수 없어요' })
    .refine((str) => !UNAVAILABLE_SITE_SLUGS.EXACT.includes(str), { message: '사용할 수 없는 사이트 주소에요' }),
};

export const cardSchema = {
  cardNumber: z
    .string({ error: '카드 번호를 입력해 주세요' })
    .transform((str) => str.replaceAll(/\D/g, ''))
    .refine((str) => str.length >= 15 && str.length <= 16, { message: '올바른 카드 번호를 입력해 주세요' }),

  expiryDate: z
    .string({ error: '만료일을 입력해 주세요' })
    .transform((str) => str.replaceAll(/\D/g, ''))
    .refine((str) => str.length === 4, { message: '올바른 만료일을 입력해 주세요' }),

  birthOrBusinessRegistrationNumber: z
    .string({ error: '생년월일 또는 사업자 등록번호를 입력해 주세요' })
    .transform((str) => str.replaceAll(/\D/g, ''))
    .transform((str) => (str.length === 8 ? str.slice(2) : str)) // YYYYMMDD -> YYMMDD
    .refine((str) => str.length === 6 || str.length === 10, {
      message: '올바른 생년월일 또는 사업자 등록번호를 입력해 주세요',
    }),

  passwordTwoDigits: z
    .string({ error: '카드 비밀번호를 입력해 주세요' })
    .regex(/^\d{2}$/, { message: '올바른 카드 비밀번호를 입력해 주세요' }),
};

export const redeemCodeSchema = z
  .string({ error: '코드를 입력해 주세요' })
  .trim()
  .toUpperCase()
  .regex(/^[A-Z0-9-]+$/, { message: '코드 형식이 맞지 않아요' })
  .transform((str) => str.replaceAll('-', '').replaceAll('O', '0').replaceAll('I', '1').replaceAll('L', '1'));
