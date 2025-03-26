import { defineCron } from '../types';

export const TestCron = defineCron('test', '* * * * *', async () => {
  // do something
});
