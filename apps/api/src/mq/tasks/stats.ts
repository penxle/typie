import { refreshLandingStats } from '#/utils/landing-stats.ts';
import { defineCron } from '../types.ts';

export const StatsLandingCron = defineCron('stats:landing', '*/30 * * * *', async () => {
  await refreshLandingStats();
});
