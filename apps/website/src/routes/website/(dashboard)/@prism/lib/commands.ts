import { disassemble } from 'es-hangul';

export type PrismCommand = { name: string; description: string; argumentHint: string | null };
export type CommandGate = 'plain' | 'ok' | 'unknown' | 'unverified';

export const commandNameOf = (text: string): string | null => {
  if (!text.startsWith('/')) return null;
  const end = text.search(/\s/);
  return end === -1 ? text.slice(1) : text.slice(1, end);
};

export const commandGate = (text: string, commands: PrismCommand[] | null): CommandGate => {
  const name = commandNameOf(text);
  if (name === null) return 'plain';
  if (commands === null) return 'unverified';
  return commands.some((command) => command.name === name) ? 'ok' : 'unknown';
};

export const commandsMatching = (commands: PrismCommand[], prefix: string): PrismCommand[] => {
  const query = disassemble(prefix);
  return commands.filter((command) => disassemble(command.name).startsWith(query));
};
