import 'dart:async';

import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:gap/gap.dart';
import 'package:sentry_flutter/sentry_flutter.dart';
import 'package:typie/context/bottom_sheet.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/context/toast.dart';
import 'package:typie/graphql/__generated__/schema.schema.gql.dart';
import 'package:typie/graphql/client.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/native/editor_native.dart';
import 'package:typie/screens/native_editor/__generated__/check_spelling_document.req.gql.dart';
import 'package:typie/screens/native_editor/state/controller.dart';
import 'package:typie/widgets/tappable.dart';

class SpellcheckSheet extends HookWidget {
  const SpellcheckSheet({
    required this.controller,
    required this.editor,
    required this.documentId,
    required this.client,
    super.key,
  });

  final EditorController controller;
  final NativeEditor editor;
  final String documentId;
  final GraphQLClient client;

  @override
  Widget build(BuildContext context) {
    final isLoading = useState(true);
    final errors = useState<List<Map<String, dynamic>>>([]);

    useEffect(() {
      Future<void> runSpellcheck() async {
        try {
          final spellcheckData = editor.getTextWithMappings();
          if (spellcheckData == null) {
            isLoading.value = false;
            return;
          }

          final text = spellcheckData['text'] as String;
          final mappings = spellcheckData['mappings'] as List<dynamic>;

          final mappingInputs = mappings.map((m) {
            final map = m as Map<String, dynamic>;
            return GSpellcheckTextMappingInput(
              (b) => b
                ..nodeId = map['nodeId'] as String
                ..textStart = map['textStart'] as int
                ..textEnd = map['textEnd'] as int
                ..blockOffset = map['blockOffset'] as int,
            );
          });

          final resp = await client.request(
            GNativeEditor_CheckSpellingDocument_MutationReq(
              (b) => b.vars.input
                ..documentId = documentId
                ..text = text
                ..mappings.addAll(mappingInputs),
            ),
          );

          final spellErrors = resp.checkSpellingDocument
              .map(
                (e) => <String, dynamic>{
                  'id': e.id,
                  'nodeId': e.nodeId,
                  'startOffset': e.startOffset,
                  'endOffset': e.endOffset,
                  'context': e.context,
                  'corrections': e.corrections.toList(),
                  'explanation': e.explanation,
                },
              )
              .toList();

          errors.value = spellErrors;

          final rawErrors = spellErrors
              .map(
                (e) => <String, dynamic>{
                  'id': e['id'],
                  'nodeId': e['nodeId'],
                  'startOffset': e['startOffset'],
                  'endOffset': e['endOffset'],
                },
              )
              .toList();
          editor.setTrackedItems(0, rawErrors);
        } catch (err) {
          unawaited(Sentry.captureException(err));
          if (context.mounted) {
            context.toast(ToastType.error, '맞춤법 검사에 실패했습니다');
          }
        } finally {
          if (context.mounted) {
            isLoading.value = false;
          }
        }
      }

      unawaited(runSpellcheck());

      return () {
        try {
          if (!editor.isDisposed) {
            editor.setTrackedItems(0, []);
          }
        } catch (_) {}
      };
    }, const []);

    final bottomPadding = MediaQuery.paddingOf(context).bottom;

    return AppBottomSheet(
      includeBottomPadding: false,
      padding: const Pad(horizontal: 20),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          if (isLoading.value) ...[
            Padding(
              padding: Pad(vertical: 20, bottom: bottomPadding + 12),
              child: const Center(child: CircularProgressIndicator()),
            ),
          ] else if (errors.value.isEmpty) ...[
            Padding(
              padding: Pad(vertical: 20, bottom: bottomPadding + 12),
              child: Center(
                child: Column(
                  spacing: 8,
                  children: [
                    Icon(LucideLightIcons.circle_check, size: 48, color: context.colors.textFaint),
                    Text('맞춤법 오류가 없습니다!', style: TextStyle(fontSize: 16, color: context.colors.textFaint)),
                  ],
                ),
              ),
            ),
          ] else ...[
            Text(
              '${errors.value.length}개의 맞춤법 오류를 발견했습니다',
              style: TextStyle(fontSize: 14, color: context.colors.textDanger),
            ),
            const Gap(12),
            ConstrainedBox(
              constraints: BoxConstraints(maxHeight: MediaQuery.sizeOf(context).height * 0.4),
              child: SingleChildScrollView(
                padding: Pad(bottom: bottomPadding + 12),
                child: Column(
                  children: errors.value
                      .map(
                        (error) => _SpellcheckErrorItem(
                          error: error,
                          onCorrect: (correction) {
                            if (controller.locked) {
                              controller.onEditBlocked?.call('locked');
                              return;
                            }

                            final errorId = error['id'] as String;
                            final range = controller.trackedItemRange(0, errorId);
                            final nodeId = range?.nodeId ?? (error['nodeId'] as String);
                            final startOffset = range?.startOffset ?? (error['startOffset'] as int);
                            final endOffset = range?.endOffset ?? (error['endOffset'] as int);

                            editor.replaceTextInBlock(nodeId, startOffset, endOffset, correction);

                            errors.value = errors.value.where((e) => e['id'] != errorId).toList();
                            editor.removeTrackedItems(0, [errorId]);
                          },
                          onTap: () {
                            final errorId = error['id'] as String;
                            final overlay = controller.state.spellcheck.overlays
                                .where((o) => o.id == errorId)
                                .firstOrNull;
                            if (overlay != null && overlay.bounds.isNotEmpty) {
                              controller.updateState(
                                (state) => state.copyWith(
                                  spellcheck: state.spellcheck.copyWith(
                                    scrollTarget: overlay.bounds.first,
                                    scrollTargetPageIdx: overlay.pageIdx,
                                  ),
                                ),
                              );
                            }
                          },
                        ),
                      )
                      .toList(),
                ),
              ),
            ),
          ],
        ],
      ),
    );
  }
}

class _SpellcheckErrorItem extends StatelessWidget {
  const _SpellcheckErrorItem({required this.error, required this.onCorrect, required this.onTap});

  final Map<String, dynamic> error;
  final void Function(String) onCorrect;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    return Tappable(
      onTap: onTap,
      child: Container(
        decoration: BoxDecoration(
          border: Border.all(color: context.colors.borderStrong),
          borderRadius: BorderRadius.circular(8),
        ),
        padding: const Pad(all: 12),
        margin: const Pad(bottom: 8),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          spacing: 8,
          children: [
            Text(error['context']?.toString() ?? '', style: TextStyle(fontSize: 14, color: context.colors.textDefault)),
            if (error['explanation'] != null)
              Text(error['explanation'].toString(), style: TextStyle(fontSize: 12, color: context.colors.textFaint)),
            Wrap(
              spacing: 8,
              runSpacing: 4,
              children: (error['corrections'] as List? ?? []).map((correction) {
                return Tappable(
                  onTap: () => onCorrect(correction.toString()),
                  child: Container(
                    decoration: BoxDecoration(
                      color: context.colors.accentDanger.withValues(alpha: 0.1),
                      border: Border.all(color: context.colors.borderStrong),
                      borderRadius: BorderRadius.circular(4),
                    ),
                    padding: const Pad(horizontal: 8, vertical: 4),
                    child: Row(
                      mainAxisSize: MainAxisSize.min,
                      spacing: 4,
                      children: [
                        Flexible(
                          child: Text(
                            correction.toString(),
                            style: TextStyle(
                              fontSize: 13,
                              fontWeight: FontWeight.w600,
                              color: context.colors.textDanger,
                            ),
                            overflow: TextOverflow.ellipsis,
                            maxLines: 1,
                          ),
                        ),
                        Icon(LucideLightIcons.arrow_right, size: 12, color: context.colors.textDanger),
                      ],
                    ),
                  ),
                );
              }).toList(),
            ),
          ],
        ),
      ),
    );
  }
}
