import crypto from 'node:crypto';

export const verifySlackSignature = (signingSecret: string, timestamp: string, signature: string, body: string) => {
  const currentTime = Math.floor(Date.now() / 1000);
  if (Number(timestamp) < currentTime - 5 * 60) {
    return false;
  }

  const sigBasestring = `v0:${timestamp}:${body}`;
  const mySignature = `v0=${crypto.createHmac('sha256', signingSecret).update(sigBasestring).digest('hex')}`;

  return crypto.timingSafeEqual(Uint8Array.from(mySignature), Uint8Array.from(signature));
};
