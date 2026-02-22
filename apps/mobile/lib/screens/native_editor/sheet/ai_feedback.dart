import 'dart:async';

import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:built_value/json_object.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:gap/gap.dart';
import 'package:typie/context/bottom_sheet.dart';
import 'package:typie/context/loader.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/graphql/__generated__/schema.schema.gql.dart';
import 'package:typie/graphql/client.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/native/editor_native.dart';
import 'package:typie/screens/native_editor/__generated__/literary_analysis_document_stream.req.gql.dart';
import 'package:typie/screens/native_editor/__generated__/update_preferences_mutation.req.gql.dart';
import 'package:typie/screens/native_editor/state/controller.dart';
import 'package:typie/widgets/tappable.dart';
import 'package:uuid/uuid.dart';

String _progressText(Map<String, dynamic>? p) {
  if (p == null) {
    return '준비 중...';
  }
  final label = (p['phase'] as String) == 'summarizing' ? '분석' : '피드백';
  return '$label 중... (${p['current']}/${p['total']})';
}

class AiFeedbackSheet extends HookWidget {
  const AiFeedbackSheet({
    required this.controller,
    required this.editor,
    required this.documentId,
    required this.client,
    required this.aiOptIn,
    super.key,
  });

  final EditorController controller;
  final NativeEditor editor;
  final String documentId;
  final GraphQLClient client;
  final bool aiOptIn;

  @override
  Widget build(BuildContext context) {
    final isLoading = useState(false);
    final hasChecked = useState(false);
    final checkFailed = useState(false);
    final feedbacks = useState<List<Map<String, dynamic>>>([]);
    final activeFeedbackId = useState<String?>(null);
    final progress = useState<Map<String, dynamic>?>(null);
    final subscription = useRef<StreamSubscription<dynamic>?>(null);

    useEffect(() {
      return () {
        unawaited(subscription.value?.cancel());
        try {
          if (!editor.isDisposed) {
            editor.setTrackedItems(1, []);
          }
        } catch (_) {}
      };
    }, const []);

    void updateOverlays() {
      final rawItems = feedbacks.value
          .map(
            (e) => <String, dynamic>{
              'id': e['id'],
              'nodeId': e['nodeId'],
              'startOffset': e['startOffset'],
              'endOffset': e['endOffset'],
            },
          )
          .toList();
      editor.setTrackedItems(1, rawItems);
    }

    Future<void> runAnalysis() async {
      if (isLoading.value) {
        return;
      }

      final spellcheckData = editor.getTextWithMappings();
      if (spellcheckData == null) {
        return;
      }

      final text = spellcheckData['text'] as String;
      if (text.trim().isEmpty) {
        return;
      }

      final mappings = spellcheckData['mappings'] as List<dynamic>;

      unawaited(subscription.value?.cancel());
      isLoading.value = true;
      hasChecked.value = true;
      checkFailed.value = false;
      feedbacks.value = [];
      activeFeedbackId.value = null;
      progress.value = null;
      editor.setTrackedItems(1, []);

      final mappingInputs = mappings.map((m) {
        final map = m as Map<String, dynamic>;
        return GDocumentTextMappingInput(
          (b) => b
            ..nodeId = map['nodeId'] as String
            ..textStart = map['textStart'] as int
            ..textEnd = map['textEnd'] as int
            ..blockOffset = map['blockOffset'] as int,
        );
      });

      subscription.value = client
          .subscribe(
            GNativeEditor_LiteraryAnalysisDocumentStream_SubscriptionReq(
              (b) => b.vars
                ..text = text
                ..mappings.addAll(mappingInputs),
            ),
          )
          .listen(
            (data) {
              final payload = data.literaryAnalysisDocumentStream;
              final type = payload.type;

              if (type == 'feedback' && payload.feedback != null) {
                final item = payload.feedback!;
                final newFeedback = <String, dynamic>{
                  'id': const Uuid().v4(),
                  'nodeId': item.nodeId,
                  'startOffset': item.startOffset,
                  'endOffset': item.endOffset,
                  'startText': item.startText,
                  'endText': item.endText,
                  'feedback': item.feedback,
                };
                feedbacks.value = [...feedbacks.value, newFeedback];
                updateOverlays();
              } else if (type == 'progress' && payload.progress != null) {
                final p = payload.progress!;
                progress.value = {'current': p.current, 'total': p.total, 'phase': p.phase};
              } else if (type == 'complete') {
                isLoading.value = false;
                progress.value = null;
              } else if (type == 'error') {
                isLoading.value = false;
                progress.value = null;
                checkFailed.value = true;
              }
            },
            onError: (_) {
              isLoading.value = false;
              progress.value = null;
              checkFailed.value = true;
            },
          );
    }

    void onFeedbackTap(Map<String, dynamic> feedback) {
      final id = feedback['id'] as String;
      activeFeedbackId.value = id;
      final overlay = controller.state.aiFeedback.overlays.where((o) => o.id == id).firstOrNull;
      if (overlay != null && overlay.bounds.isNotEmpty) {
        controller.updateState(
          (state) => state.copyWith(
            aiFeedback: state.aiFeedback.copyWith(
              scrollTarget: overlay.bounds.first,
              scrollTargetPageIdx: overlay.pageIdx,
            ),
          ),
        );
      }
    }

    void onDismissFeedback(Map<String, dynamic> feedback) {
      final id = feedback['id'] as String;
      feedbacks.value = feedbacks.value.where((f) => f['id'] != id).toList();
      if (activeFeedbackId.value == id) {
        activeFeedbackId.value = null;
      }
      editor.removeTrackedItems(1, [id]);
    }

    final bottomPadding = MediaQuery.paddingOf(context).bottom;

    if (!aiOptIn) {
      return AppBottomSheet(
        includeBottomPadding: false,
        padding: const Pad(horizontal: 20),
        child: Padding(
          padding: Pad(vertical: 40, bottom: bottomPadding + 12),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              Container(
                width: 64,
                height: 64,
                decoration: BoxDecoration(color: context.colors.surfaceMuted, borderRadius: BorderRadius.circular(16)),
                child: Icon(LucideLightIcons.lightbulb, size: 28, color: context.colors.textFaint),
              ),
              const Gap(20),
              Text(
                'AI 기능을 사용하려면\n활성화가 필요해요',
                textAlign: TextAlign.center,
                style: TextStyle(fontSize: 13, color: context.colors.textFaint),
              ),
              const Gap(20),
              Tappable(
                onTap: () async {
                  await context.showBottomSheet(
                    child: ConfirmBottomSheet(
                      title: 'AI 기능을 활성화하시겠어요?',
                      confirmText: '활성화',
                      onConfirm: () async {
                        await context.runWithLoader(() async {
                          await client.request(
                            GNativeEditor_UpdatePreferences_MutationReq(
                              (b) => b..vars.input.value = JsonObject({'aiOptIn': true}),
                            ),
                          );
                        });
                      },
                      child: const _AiOptInNotice(),
                    ),
                  );
                },
                child: Container(
                  padding: const Pad(horizontal: 16, vertical: 8),
                  decoration: BoxDecoration(
                    color: context.colors.surfaceInverse,
                    borderRadius: BorderRadius.circular(8),
                  ),
                  child: Text(
                    'AI 기능 활성화',
                    style: TextStyle(fontSize: 14, fontWeight: FontWeight.w600, color: context.colors.textInverse),
                  ),
                ),
              ),
            ],
          ),
        ),
      );
    }

    return AppBottomSheet(
      includeBottomPadding: false,
      padding: const Pad(horizontal: 20),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          if (!hasChecked.value && !isLoading.value) ...[
            Padding(
              padding: Pad(vertical: 40, bottom: bottomPadding + 12),
              child: Column(
                mainAxisSize: MainAxisSize.min,
                children: [
                  Container(
                    width: 64,
                    height: 64,
                    decoration: BoxDecoration(
                      color: context.colors.surfaceMuted,
                      borderRadius: BorderRadius.circular(16),
                    ),
                    child: Icon(LucideLightIcons.lightbulb, size: 28, color: context.colors.textFaint),
                  ),
                  const Gap(20),
                  Text(
                    '글에 대한 AI 피드백을\n받아보세요',
                    textAlign: TextAlign.center,
                    style: TextStyle(fontSize: 13, color: context.colors.textFaint),
                  ),
                  const Gap(20),
                  Tappable(
                    onTap: runAnalysis,
                    child: Container(
                      padding: const Pad(horizontal: 16, vertical: 8),
                      decoration: BoxDecoration(
                        border: Border.all(color: context.colors.borderStrong),
                        borderRadius: BorderRadius.circular(8),
                      ),
                      child: Text('분석 시작', style: TextStyle(fontSize: 14, color: context.colors.textDefault)),
                    ),
                  ),
                ],
              ),
            ),
          ] else if (checkFailed.value) ...[
            Padding(
              padding: Pad(vertical: 40, bottom: bottomPadding + 12),
              child: Column(
                mainAxisSize: MainAxisSize.min,
                children: [
                  Icon(LucideLightIcons.circle_alert, size: 32, color: context.colors.textFaint),
                  const Gap(8),
                  Text('분석에 실패했습니다', style: TextStyle(fontSize: 16, color: context.colors.textFaint)),
                  const Gap(4),
                  Text('잠시 후 다시 시도해주세요', style: TextStyle(fontSize: 14, color: context.colors.textFaint)),
                ],
              ),
            ),
          ] else ...[
            Row(
              mainAxisAlignment: MainAxisAlignment.spaceBetween,
              children: [
                Row(
                  children: [
                    Text(
                      'AI 피드백',
                      style: TextStyle(fontSize: 13, fontWeight: FontWeight.w600, color: context.colors.textSubtle),
                    ),
                    if (!isLoading.value && feedbacks.value.isNotEmpty) ...[
                      const Gap(6),
                      Container(
                        padding: const Pad(horizontal: 6, vertical: 2),
                        decoration: BoxDecoration(
                          color: context.colors.accentBrand.withValues(alpha: 0.1),
                          borderRadius: BorderRadius.circular(4),
                        ),
                        child: Text(
                          '${feedbacks.value.length}',
                          style: TextStyle(
                            fontSize: 11,
                            fontWeight: FontWeight.w600,
                            color: context.colors.accentBrand,
                          ),
                        ),
                      ),
                    ],
                  ],
                ),
                if (!isLoading.value && hasChecked.value)
                  Tappable(
                    onTap: runAnalysis,
                    child: Text(
                      '다시 분석',
                      style: TextStyle(fontSize: 13, fontWeight: FontWeight.w500, color: context.colors.textFaint),
                    ),
                  ),
              ],
            ),
            const Gap(12),
            if (!isLoading.value && feedbacks.value.isEmpty) ...[
              Padding(
                padding: Pad(vertical: 24, bottom: bottomPadding + 12),
                child: Column(
                  children: [
                    Icon(LucideLightIcons.circle_check, size: 32, color: context.colors.textFaint),
                    const Gap(8),
                    Text('피드백이 없습니다', style: TextStyle(fontSize: 16, color: context.colors.textFaint)),
                  ],
                ),
              ),
            ] else ...[
              ConstrainedBox(
                constraints: BoxConstraints(maxHeight: MediaQuery.sizeOf(context).height * 0.4),
                child: SingleChildScrollView(
                  padding: Pad(bottom: bottomPadding + 12),
                  child: Column(
                    children: [
                      ...feedbacks.value.map(
                        (feedback) => _AiFeedbackItem(
                          feedback: feedback,
                          isActive: activeFeedbackId.value == feedback['id'],
                          onTap: () => onFeedbackTap(feedback),
                          onDismiss: () => onDismissFeedback(feedback),
                        ),
                      ),
                      if (isLoading.value)
                        Padding(
                          padding: const Pad(vertical: 16),
                          child: Row(
                            mainAxisAlignment: MainAxisAlignment.center,
                            children: [
                              SizedBox(
                                width: 16,
                                height: 16,
                                child: CircularProgressIndicator(strokeWidth: 2, color: context.colors.textFaint),
                              ),
                              const Gap(8),
                              Text(
                                _progressText(progress.value),
                                style: TextStyle(fontSize: 13, color: context.colors.textFaint),
                              ),
                            ],
                          ),
                        ),
                    ],
                  ),
                ),
              ),
            ],
          ],
        ],
      ),
    );
  }
}

class _AiFeedbackItem extends StatelessWidget {
  const _AiFeedbackItem({required this.feedback, required this.isActive, required this.onTap, required this.onDismiss});

  final Map<String, dynamic> feedback;
  final bool isActive;
  final VoidCallback onTap;
  final VoidCallback onDismiss;

  @override
  Widget build(BuildContext context) {
    final startText = feedback['startText'] as String;
    final endText = feedback['endText'] as String;
    final feedbackText = feedback['feedback'] as String;

    return Tappable(
      onTap: onTap,
      child: Container(
        decoration: BoxDecoration(
          border: Border.all(color: isActive ? context.colors.accentBrand : context.colors.borderDefault),
          borderRadius: BorderRadius.circular(8),
        ),
        padding: const Pad(all: 12),
        margin: const Pad(bottom: 8),
        child: Stack(
          children: [
            Padding(
              padding: const Pad(right: 24),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                spacing: 8,
                children: [
                  Text(
                    startText == endText ? '"$startText"' : '"$startText" ... "$endText"',
                    style: TextStyle(fontSize: 14, color: context.colors.textDefault),
                  ),
                  Text(
                    feedbackText,
                    style: TextStyle(fontSize: 12, color: context.colors.textFaint),
                    maxLines: isActive ? null : 2,
                    overflow: isActive ? null : TextOverflow.ellipsis,
                  ),
                ],
              ),
            ),
            Positioned(
              top: 0,
              right: 0,
              child: Tappable(
                onTap: onDismiss,
                child: Container(
                  padding: const Pad(all: 4),
                  decoration: BoxDecoration(borderRadius: BorderRadius.circular(4)),
                  child: Icon(LucideLightIcons.x, size: 14, color: context.colors.textFaint),
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _AiOptInNotice extends StatelessWidget {
  const _AiOptInNotice();

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          '타이피는 사용자의 프라이버시를 최우선으로 생각해요. 사용자가 작성한 글은 어떠한 경우에도 AI 모델 학습에 사용되지 않아요.',
          style: TextStyle(fontSize: 14, color: context.colors.textFaint),
        ),
        const Gap(16),
        Container(
          padding: const Pad(all: 12),
          decoration: BoxDecoration(color: context.colors.surfaceMuted, borderRadius: BorderRadius.circular(8)),
          child: const Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            spacing: 8,
            children: [
              _AiOptInNoticeItem(title: '학습 금지', description: '사용자의 글은 AI 모델 학습이나 개선에 절대 사용되지 않아요.'),
              _AiOptInNoticeItem(title: '요청 시에만', description: '사용자가 요청하지 않는 한 타이피가 임의로 AI를 사용하지 않아요.'),
              _AiOptInNoticeItem(title: '투명한 처리', description: 'AI가 언제, 어떻게 사용되는지 사용자가 항상 알 수 있어요.'),
              _AiOptInNoticeItem(title: '완전한 통제', description: 'AI 기능은 언제든 끌 수 있고, 비활성화하면 어떤 AI 처리도 일어나지 않아요.'),
              _AiOptInNoticeItem(title: '권리 보장', description: '타이피는 사용자 창작물에 대한 어떤 권리도 주장하지 않아요.'),
            ],
          ),
        ),
      ],
    );
  }
}

class _AiOptInNoticeItem extends StatelessWidget {
  const _AiOptInNoticeItem({required this.title, required this.description});

  final String title;
  final String description;

  @override
  Widget build(BuildContext context) {
    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text('• ', style: TextStyle(fontSize: 13, color: context.colors.textFaint)),
        Expanded(
          child: RichText(
            text: TextSpan(
              style: TextStyle(fontSize: 13, color: context.colors.textFaint),
              children: [
                TextSpan(
                  text: '$title: ',
                  style: const TextStyle(fontWeight: FontWeight.w600),
                ),
                TextSpan(text: description),
              ],
            ),
          ),
        ),
      ],
    );
  }
}
