type Base64Alphabet = 'base64' | 'base64url';
type LastChunkHandling = 'loose' | 'strict' | 'stop-before-partial';

type ToBase64Options = {
  alphabet?: Base64Alphabet;
  omitPadding?: boolean;
};

type FromBase64Options = {
  alphabet?: Base64Alphabet;
  lastChunkHandling?: LastChunkHandling;
};

const BASE64_CHARS = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/';

const hasNativeSupport = typeof Uint8Array.prototype.toBase64 === 'function' && typeof Uint8Array.fromBase64 === 'function';

if (!hasNativeSupport) {
  if (!Uint8Array.prototype.toBase64) {
    Uint8Array.prototype.toBase64 = function (options: ToBase64Options = {}): string {
      const { alphabet = 'base64', omitPadding = false } = options;

      const CHUNK_SIZE = 0x7f_ff;
      let result = '';

      if (this.length <= CHUNK_SIZE) {
        // @ts-expect-error - Using apply with typed array for performance
        result = btoa(String.fromCharCode.apply(null, this));
      } else {
        let binaryString = '';
        for (let i = 0; i < this.length; i += CHUNK_SIZE) {
          const end = Math.min(i + CHUNK_SIZE, this.length);
          const chunk = this.subarray(i, end);
          // @ts-expect-error - Using apply with typed array for performance
          binaryString += String.fromCharCode.apply(null, chunk);
        }
        result = btoa(binaryString);
      }

      if (alphabet === 'base64url') {
        result = result.replaceAll('+', '-').replaceAll('/', '_');
      }

      if (omitPadding) {
        const paddingStart = result.indexOf('=');
        if (paddingStart !== -1) {
          result = result.slice(0, Math.max(0, paddingStart));
        }
      }

      return result;
    };
  }

  if (!Uint8Array.fromBase64) {
    Uint8Array.fromBase64 = function (string: string, options: FromBase64Options = {}): Uint8Array<ArrayBuffer> {
      const { alphabet = 'base64', lastChunkHandling = 'loose' } = options;

      let cleanString = string;
      if (/\s/.test(string)) {
        cleanString = string.replaceAll(/\s+/g, '');
      }

      if (alphabet === 'base64url') {
        cleanString = cleanString.replaceAll('-', '+').replaceAll('_', '/');
      }

      const mod = cleanString.length & 3;

      if (lastChunkHandling === 'stop-before-partial' && mod > 0) {
        cleanString = cleanString.slice(0, cleanString.length - mod);
      } else if (lastChunkHandling === 'loose' && mod > 0) {
        cleanString += mod === 2 ? '==' : '=';
      } else if (lastChunkHandling === 'strict') {
        if (mod !== 0) {
          throw new SyntaxError('Invalid base64 string: length not multiple of 4');
        }

        if (!/^[A-Za-z0-9+/]*={0,2}$/.test(cleanString)) {
          throw new SyntaxError('Invalid base64 string: contains invalid characters');
        }

        const paddingIndex = cleanString.indexOf('=');
        if (paddingIndex !== -1) {
          const lastChar = cleanString[paddingIndex - 1];
          const lastValue = BASE64_CHARS.indexOf(lastChar);
          const numPadding = cleanString.length - paddingIndex;

          if ((numPadding === 1 && (lastValue & 0x03) !== 0) || (numPadding === 2 && (lastValue & 0x0f) !== 0)) {
            throw new SyntaxError('Invalid base64 string: non-zero overflow bits');
          }
        }
      }

      try {
        const binaryString = atob(cleanString);
        const len = binaryString.length;

        const bytes = new Uint8Array(len);
        for (let i = 0; i < len; i++) {
          // eslint-disable-next-line unicorn/prefer-code-point
          bytes[i] = binaryString.charCodeAt(i);
        }

        return bytes;
      } catch (err) {
        throw new SyntaxError(`Invalid base64 string: ${err instanceof Error ? err.message : 'decode failed'}`);
      }
    };
  }
}

declare global {
  // eslint-disable-next-line @typescript-eslint/consistent-type-definitions
  interface Uint8Array {
    toBase64(options?: ToBase64Options): string;
  }

  // eslint-disable-next-line @typescript-eslint/consistent-type-definitions
  interface Uint8ArrayConstructor {
    fromBase64(string: string, options?: FromBase64Options): Uint8Array<ArrayBuffer>;
  }
}
