import { assemble, disassemble } from 'es-hangul';
import type { TransitionConfig } from 'svelte/transition';

const getText = (n: Node): Text[] => {
  if (n.nodeType === Node.TEXT_NODE) return [n as Text];
  if (!n.childNodes) return [];
  return [...n.childNodes].flatMap(getText);
};

const HANGUL_REGEX = /[가-힣]/;
const PART_REGEX = /[ㄱ-ㅎㅏ-ㅣ]/;

export const typewriter = (node: Element, { delay = 0, speed = 50 } = {}): TransitionConfig => {
  const textNodes = getText(node);
  if (textNodes.length === 0) throw new Error('Typewriter requires text nodes');

  let totalParts = 0;
  const nodeRanges = textNodes.map((textNode) => {
    const originalText = textNode.textContent ?? '';

    const characterParts: string[][] = [];
    let partCount = 0;

    for (const character of originalText) {
      const parts = HANGUL_REGEX.test(character) ? [...disassemble(character)] : [character];

      characterParts.push(parts);
      partCount += parts.length;
    }

    const range: [number, number] = [totalParts, totalParts + partCount];
    totalParts += partCount;

    return {
      textNode,
      range,
      originalText,
      characterParts,
    };
  });

  let currentNodeIndex = 0;

  return {
    delay,
    duration: totalParts * speed,
    tick: (t: number, u: number) => {
      const progress = t;
      const direction = u <= 0.5 ? 0 : 1; // 0 = outro, 1 = intro

      // 초기화: intro 시작 또는 outro 종료 시
      if ((direction === 1 && progress === 0) || (direction === 0 && progress === 1)) {
        nodeRanges.forEach((nodeRange) => {
          nodeRange.textNode.textContent = direction === 1 ? '' : nodeRange.originalText;
        });
        currentNodeIndex = 0;
      }

      const position = Math.trunc(totalParts * progress);

      // intro(1)와 outro(0)에 따라 처리 분기
      if (direction === 1) {
        // Intro: 텍스트를 점진적으로 채움

        // 완성된 노드 처리
        while (currentNodeIndex < nodeRanges.length - 1 && position >= nodeRanges[currentNodeIndex].range[1]) {
          nodeRanges[currentNodeIndex].textNode.textContent = nodeRanges[currentNodeIndex].originalText;
          currentNodeIndex++;
        }

        const currentNode = nodeRanges[currentNodeIndex];
        const displayedParts = position - currentNode.range[0];

        if (displayedParts <= 0) {
          currentNode.textNode.textContent = '';
          return;
        }

        let result = '';
        let processedParts = 0;

        for (const parts of currentNode.characterParts) {
          if (processedParts >= displayedParts) break;

          const visibleParts = Math.min(parts.length, displayedParts - processedParts);
          if (visibleParts <= 0) break;

          const isHangulPart = parts.length > 1 && PART_REGEX.test(parts[0]);
          const selectedParts = visibleParts < parts.length ? parts.slice(0, visibleParts) : parts;

          result += isHangulPart ? assemble(selectedParts) : selectedParts[0];
          processedParts += parts.length;
        }

        currentNode.textNode.textContent = result;
      } else {
        // Outro: 텍스트를 점진적으로 지움

        // 완전히 지워진 노드 처리
        while (currentNodeIndex < nodeRanges.length - 1 && position < nodeRanges[currentNodeIndex].range[0]) {
          nodeRanges[currentNodeIndex].textNode.textContent = '';
          currentNodeIndex++;
        }

        const currentNode = nodeRanges[currentNodeIndex];
        const remainingParts = position - currentNode.range[0];

        if (remainingParts <= 0) {
          currentNode.textNode.textContent = '';
          return;
        }

        let result = '';
        let processedParts = 0;

        for (const parts of currentNode.characterParts) {
          if (processedParts >= remainingParts) {
            // 남은 텍스트를 완전히 제거
            break;
          }

          const visibleParts = Math.min(parts.length, remainingParts - processedParts);

          if (visibleParts > 0) {
            const isHangulPart = parts.length > 1 && PART_REGEX.test(parts[0]);
            const selectedParts = visibleParts < parts.length ? parts.slice(0, visibleParts) : parts;

            result += isHangulPart ? assemble(selectedParts) : selectedParts[0];
          }

          processedParts += parts.length;
        }

        currentNode.textNode.textContent = result;
      }
    },
  };
};
