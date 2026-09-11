import { detectPlatform } from './platform';
import type { PlatformId } from './platform';

export const detectedPlatform = () => {
  let detected = $state<PlatformId | null>(null);

  $effect(() => {
    detected = detectPlatform();
  });

  return {
    get current() {
      return detected;
    },
  };
};
