import { eq } from 'drizzle-orm';
import { buildRoster } from '$lib/domain/participants.ts';
import { EvaluatorConsents } from './db/index.ts';
import type { RosterEntry } from '$lib/domain/participants.ts';
import type { createDb } from './db/index.ts';

type Db = ReturnType<typeof createDb>;

// 어드민 배지가 필요한 명단 화면 전용 — 배정·집계는 adminEmails를 보지 않는다.
export const listRoster = async (db: Db, adminEmails = ''): Promise<RosterEntry[]> =>
  buildRoster(
    await db.select({ email: EvaluatorConsents.email, evaluating: EvaluatorConsents.evaluating }).from(EvaluatorConsents),
    adminEmails,
  );

export const listParticipants = async (db: Db): Promise<string[]> => {
  const rows = await db.select({ email: EvaluatorConsents.email }).from(EvaluatorConsents).where(eq(EvaluatorConsents.evaluating, true));
  return rows.map((r) => r.email);
};

// requiredJudgments가 null인 태스크(중복 구간)의 분모. 0명이면 나눗셈과 진행률이 깨지므로 1로 받친다.
export const countParticipants = async (db: Db): Promise<number> => {
  const participants = await listParticipants(db);
  return Math.max(1, participants.length);
};
