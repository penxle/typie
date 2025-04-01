import { z } from 'zod';

export const userSchema = {
  name: z.string().trim().min(1, '이름을 입력해주세요').max(20, '이름은 20자를 넘을 수 없어요'),
};
