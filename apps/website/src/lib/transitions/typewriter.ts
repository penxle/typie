import { assemble, disassemble } from 'es-hangul';
import type { TransitionConfig } from 'svelte/transition';

type NodeInfo = {
  textNode: Text;
  range: [number, number];
  originalText: string;
  characterParts: string[][];
};

const HANGUL_REGEX = /[가-힣]/;
const PART_REGEX = /[ㄱ-ㅎㅏ-ㅣ]/;

const getText = (node: Node): Text[] => (node.nodeType === Node.TEXT_NODE ? [node as Text] : [...(node.childNodes || [])].flatMap(getText));

export const typewriter = (node: Element, { delay = 0, speed = 50 } = {}): TransitionConfig => {
  const textNodes = getText(node);
  if (textNodes.length === 0) throw new Error('Typewriter requires text nodes');

  let totalParts = 0;
  const nodeRanges: NodeInfo[] = textNodes.map((textNode) => {
    const originalText = textNode.textContent ?? '';
    const characterParts: string[][] = [];
    let partCount = 0;

    for (const char of originalText) {
      const parts = HANGUL_REGEX.test(char) ? [...disassemble(char)] : [char];
      characterParts.push(parts);
      partCount += parts.length;
    }

    const startPos = totalParts;
    totalParts += partCount;

    return {
      textNode,
      originalText,
      characterParts,
      range: [startPos, totalParts],
    };
  });

  let currentNodeIndex = 0;

  return {
    delay,
    duration: totalParts * (1000 / speed),
    tick: (t, u) => {
      const isIntro = u > 0.5;
      const position = Math.floor(totalParts * t);

      if ((isIntro && t === 0) || (!isIntro && t === 1)) {
        nodeRanges.forEach(({ textNode, originalText }) => {
          textNode.textContent = isIntro ? '' : originalText;
        });
        currentNodeIndex = 0;
        return;
      }

      const compareFunc = isIntro ? (node: NodeInfo) => position >= node.range[1] : (node: NodeInfo) => position < node.range[0];

      while (currentNodeIndex < nodeRanges.length - 1 && compareFunc(nodeRanges[currentNodeIndex])) {
        const { textNode, originalText } = nodeRanges[currentNodeIndex];
        textNode.textContent = isIntro ? originalText : '';
        currentNodeIndex++;
      }

      const currentNode = nodeRanges[currentNodeIndex];
      const visibleParts = position - currentNode.range[0];

      if (visibleParts <= 0) {
        currentNode.textNode.textContent = '';
        return;
      }

      let result = '';
      let processedParts = 0;

      for (const parts of currentNode.characterParts) {
        if (processedParts >= visibleParts) break;

        const partsToShow = Math.min(parts.length, visibleParts - processedParts);
        if (partsToShow <= 0) break;

        result +=
          parts.length > 1 && PART_REGEX.test(parts[0])
            ? assemble(partsToShow < parts.length ? parts.slice(0, partsToShow) : parts)
            : parts[0];

        processedParts += parts.length;
      }

      currentNode.textNode.textContent = result;
    },
  };
};
