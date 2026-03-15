import { defineCron } from '../types.ts';

export const TestCron = defineCron('test', '* * * * *', async () => {
  // do something
});
