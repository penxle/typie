import { Extension } from '@tiptap/core';
import { Node } from '@tiptap/pm/model';
import { Selection, TextSelection } from '@tiptap/pm/state';

export const SentenceNavigation = Extension.create({
  name: 'sentenceNavigation',

  addKeyboardShortcuts() {
    const getTextFromNode = (node: Node) => {
      return node.textBetween(0, node.nodeSize - 2, '\n', (n: Node) => {
        if (n.type.name === 'hard_break') {
          return '\n';
        }
        return '';
      });
    };

    const findAdjacentParagraph = (
      doc: Node,
      currentParagraphStart: number,
      direction: 'up' | 'down',
    ): { found: boolean; pos?: number } => {
      const $from = doc.resolve(currentParagraphStart);
      const currentBoundary = direction === 'up' ? currentParagraphStart : $from.end();

      if (direction === 'up') {
        const $before = doc.resolve(currentBoundary - 1);

        if ($before.nodeBefore?.type.name === 'paragraph') {
          return {
            found: true,
            pos: currentBoundary - 2, // 이전 문단의 끝 위치
          };
        }

        let searchPos = currentBoundary - 2;

        while (searchPos > 0) {
          const $pos = doc.resolve(searchPos);

          if ($pos.parent.type.name === 'paragraph' && $pos.end() < currentBoundary) {
            return {
              found: true,
              pos: $pos.end(), // 이전 문단의 끝 위치
            };
          }

          searchPos--;
        }
      } else {
        const $after = doc.resolve(currentBoundary);

        if ($after.nodeAfter?.type.name === 'paragraph') {
          return {
            found: true,
            pos: currentBoundary + 2, // 다음 문단의 시작 위치
          };
        }

        let searchPos = currentBoundary + 2;

        while (searchPos < doc.content.size) {
          const $pos = doc.resolve(searchPos);

          if ($pos.parent.type.name === 'paragraph' && $pos.start() > currentBoundary) {
            return {
              found: true,
              pos: $pos.start(), // 다음 문단의 시작 위치
            };
          }

          searchPos++;
        }
      }

      return { found: false };
    };

    const findSentenceBoundaryInParagraph = (paragraphText: string, localPos: number, direction: 'up' | 'down') => {
      if (Intl.Segmenter === undefined) {
        return null;
      }

      const locale = navigator.language || 'en';
      const segmenter = new Intl.Segmenter(locale, { granularity: 'sentence' });
      const segments = [...segmenter.segment(paragraphText)];

      if (segments.length === 0) return null;

      let accumulatedLength = 0;

      for (let i = 0; i < segments.length; i++) {
        const segment = segments[i];
        const segmentStart = accumulatedLength;
        const segmentEnd = accumulatedLength + segment.segment.length;

        if (direction === 'up') {
          // 현재 위치가 이 문장 범위 내에 있는지 확인
          if (localPos <= segmentEnd) {
            const isAtSentenceStart = localPos <= segmentStart;

            if (isAtSentenceStart) {
              if (i > 0) {
                // 이전 문장의 시작으로 이동
                return accumulatedLength - segments[i - 1].segment.length;
              } else {
                // 첫 문장이면 이전 문단으로
                return null;
              }
            } else {
              // 현재 문장의 시작으로 이동
              return segmentStart;
            }
          }
        } else {
          // 현재 위치가 이 문장 범위 내에 있는지 확인
          if (localPos < segmentEnd) {
            // 뒤쪽 공백 제외한 문장 끝 위치 계산
            const trimmedEnd = segmentStart + segment.segment.trimEnd().length;
            const isAtSentenceEnd = localPos >= trimmedEnd;

            if (isAtSentenceEnd) {
              if (i < segments.length - 1) {
                // 다음 문장의 끝으로 이동
                const nextSegment = segments[i + 1];
                const nextSegmentStart = segmentEnd;
                const nextTrimmedEnd = nextSegmentStart + nextSegment.segment.trimEnd().length;
                return nextTrimmedEnd;
              } else {
                // 마지막 문장이면 다음 문단으로
                return null;
              }
            } else {
              // 현재 문장의 끝으로 이동
              return trimmedEnd;
            }
          }
        }

        accumulatedLength += segment.segment.length;
      }

      return null;
    };

    return {
      'Alt-ArrowUp': ({ editor }) => {
        const { state } = editor;
        const { doc, selection } = state;
        const { $from } = selection;

        const paragraph = $from.parent;
        const paragraphStart = $from.start();
        const localPos = $from.pos - paragraphStart;

        const paragraphText = getTextFromNode(paragraph);
        const targetLocalPos = findSentenceBoundaryInParagraph(paragraphText, localPos, 'up');
        let targetPos;

        if (targetLocalPos === null) {
          const adjacentParagraph = findAdjacentParagraph(doc, paragraphStart, 'up');

          if (adjacentParagraph.found && adjacentParagraph.pos !== undefined) {
            targetPos = adjacentParagraph.pos;
          } else {
            targetPos = Selection.atStart(doc).from;
          }
        } else {
          targetPos = paragraphStart + targetLocalPos;
        }

        return editor.chain().focus().setTextSelection(targetPos).scrollIntoView().run();
      },

      'Alt-ArrowDown': ({ editor }) => {
        const { state } = editor;
        const { doc, selection } = state;
        const { $from } = selection;

        const paragraph = $from.parent;
        const paragraphStart = $from.start();
        const localPos = $from.pos - paragraphStart;

        const paragraphText = getTextFromNode(paragraph);
        const targetLocalPos = findSentenceBoundaryInParagraph(paragraphText, localPos, 'down');
        let targetPos;

        if (targetLocalPos === null) {
          const adjacentParagraph = findAdjacentParagraph(doc, paragraphStart, 'down');

          if (adjacentParagraph.found && adjacentParagraph.pos !== undefined) {
            targetPos = adjacentParagraph.pos;
          } else {
            targetPos = Selection.atEnd(doc).to;
          }
        } else {
          targetPos = paragraphStart + targetLocalPos;
        }

        return editor.chain().focus().setTextSelection(targetPos).scrollIntoView().run();
      },

      'Shift-Alt-ArrowUp': ({ editor }) => {
        const { state } = editor;
        const { doc, selection } = state;
        const { $anchor, $head } = selection;

        const $movingPos = $head;
        const paragraph = $movingPos.parent;
        const paragraphStart = $movingPos.start();
        const localPos = $movingPos.pos - paragraphStart;

        const paragraphText = getTextFromNode(paragraph);
        const targetLocalPos = findSentenceBoundaryInParagraph(paragraphText, localPos, 'up');

        let targetPos: number;

        if (targetLocalPos === null) {
          const adjacentParagraph = findAdjacentParagraph(doc, paragraphStart, 'up');

          if (adjacentParagraph.found && adjacentParagraph.pos !== undefined) {
            targetPos = adjacentParagraph.pos;
          } else {
            targetPos = Selection.atStart(doc).from;
          }
        } else {
          targetPos = paragraphStart + targetLocalPos;
        }

        return editor
          .chain()
          .focus()
          .command(({ tr, dispatch }) => {
            if (dispatch) {
              const newSelection = TextSelection.between(doc.resolve($anchor.pos), doc.resolve(targetPos));
              tr.setSelection(newSelection);
              tr.scrollIntoView();
              dispatch(tr);
            }
            return true;
          })
          .scrollIntoView()
          .run();
      },

      'Shift-Alt-ArrowDown': ({ editor }) => {
        const { state } = editor;
        const { doc, selection } = state;
        const { $anchor, $head } = selection;

        const $movingPos = $head;
        const paragraph = $movingPos.parent;
        const paragraphStart = $movingPos.start();
        const localPos = $movingPos.pos - paragraphStart;

        const paragraphText = getTextFromNode(paragraph);
        const targetLocalPos = findSentenceBoundaryInParagraph(paragraphText, localPos, 'down');

        let targetPos: number;

        if (targetLocalPos === null) {
          const adjacentParagraph = findAdjacentParagraph(doc, paragraphStart, 'down');

          if (adjacentParagraph.found && adjacentParagraph.pos !== undefined) {
            targetPos = adjacentParagraph.pos;
          } else {
            targetPos = Selection.atEnd(doc).to;
          }
        } else {
          targetPos = paragraphStart + targetLocalPos;
        }

        return editor
          .chain()
          .focus()
          .command(({ tr, dispatch }) => {
            if (dispatch) {
              const newSelection = TextSelection.between(doc.resolve($anchor.pos), doc.resolve(targetPos));
              tr.setSelection(newSelection);
              tr.scrollIntoView();
              dispatch(tr);
            }
            return true;
          })
          .scrollIntoView()
          .run();
      },
    };
  },
});
