import 'dart:async';

import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:auto_route/auto_route.dart';
import 'package:built_value/json_object.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:gap/gap.dart';
import 'package:mixpanel_flutter/mixpanel_flutter.dart';
import 'package:typie/context/bottom_sheet.dart';
import 'package:typie/context/loader.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/context/toast.dart';
import 'package:typie/graphql/client.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/screens/editor/__generated__/update_preferences_mutation.req.gql.dart';
import 'package:typie/screens/editor/scope.dart';
import 'package:typie/widgets/tappable.dart';

class AiFeedback {
  AiFeedback({
    required this.id,
    required this.from,
    required this.to,
    required this.startText,
    required this.endText,
    required this.feedback,
  });

  final String id;
  final int from;
  final int to;
  final String startText;
  final String endText;
  final String feedback;
}

class AiFeedbackProgress {
  AiFeedbackProgress({required this.current, required this.total, required this.phase});

  final int current;
  final int total;
  final String phase;
}

class AiFeedbackBottomSheet extends HookWidget {
  const AiFeedbackBottomSheet({required this.scope, required this.aiOptIn, super.key});

  final EditorStateScope scope;
  final bool aiOptIn;

  @override
  Widget build(BuildContext context) {
    final client = useService<GraphQLClient>();
    final mixpanel = useService<Mixpanel>();
    final webViewController = useValueListenable(scope.webViewController);

    final isLoading = useState(false);
    final hasChecked = useState(false);
    final checkFailed = useState(false);
    final feedbacks = useState<List<AiFeedback>>([]);
    final activeFeedbackId = useState<String?>(null);
    final progress = useState<AiFeedbackProgress?>(null);

    useEffect(() {
      if (webViewController == null) {
        return null;
      }

      final subscription = webViewController.onEvent.listen((event) {
        if (event.name == 'aiFeedbackUpdate') {
          final data = event.data as Map<String, dynamic>;
          final type = data['type'] as String;

          if (type == 'feedback') {
            final feedbackData = data['feedback'] as Map<String, dynamic>;
            final newFeedback = AiFeedback(
              id: feedbackData['id'] as String,
              from: feedbackData['from'] as int,
              to: feedbackData['to'] as int,
              startText: feedbackData['startText'] as String,
              endText: feedbackData['endText'] as String,
              feedback: feedbackData['feedback'] as String,
            );
            feedbacks.value = [...feedbacks.value, newFeedback];
          } else if (type == 'progress') {
            final progressData = data['progress'] as Map<String, dynamic>;
            progress.value = AiFeedbackProgress(
              current: progressData['current'] as int,
              total: progressData['total'] as int,
              phase: progressData['phase'] as String,
            );
          } else if (type == 'complete') {
            isLoading.value = false;
            progress.value = null;
            unawaited(
              mixpanel.track('ai-feedback', properties: {'feedbacks': feedbacks.value.length, 'via': 'editor'}),
            );
          } else if (type == 'error') {
            isLoading.value = false;
            progress.value = null;
            checkFailed.value = true;
          }
        }
      });

      return subscription.cancel;
    }, [webViewController]);

    useEffect(() {
      return () {
        unawaited(scope.webViewController.value?.callProcedure('setAiFeedbackHighlight', {'id': null}));
        unawaited(scope.webViewController.value?.callProcedure('stopAiFeedback'));
      };
    }, []);

    Future<void> runAnalysis() async {
      if (webViewController == null || isLoading.value) {
        return;
      }

      isLoading.value = true;
      hasChecked.value = true;
      checkFailed.value = false;
      feedbacks.value = [];
      activeFeedbackId.value = null;
      progress.value = null;

      try {
        await webViewController.callProcedure('runAiFeedback');
      } catch (err) {
        if (context.mounted) {
          context.toast(ToastType.error, 'AI 피드백 분석에 실패했습니다');
          await context.router.root.maybePop();
        }
      }
    }

    Future<void> onFeedbackTap(AiFeedback feedback) async {
      if (activeFeedbackId.value == feedback.id) {
        activeFeedbackId.value = null;
        await webViewController?.callProcedure('setAiFeedbackHighlight', {'id': null});
      } else {
        activeFeedbackId.value = feedback.id;
        await webViewController?.callProcedure('setAiFeedbackHighlight', {'id': feedback.id});
        await webViewController?.callProcedure('scrollToAiFeedback', {'id': feedback.id});
      }
    }

    Future<void> onDismissFeedback(AiFeedback feedback) async {
      feedbacks.value = feedbacks.value.where((f) => f.id != feedback.id).toList();
      if (activeFeedbackId.value == feedback.id) {
        activeFeedbackId.value = null;
        await webViewController?.callProcedure('setAiFeedbackHighlight', {'id': null});
      }
      await webViewController?.callProcedure('dismissAiFeedback', {'id': feedback.id});
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
                            GAiFeedbackBottomSheet_UpdatePreferences_MutationReq(
                              (b) => b..vars.input.value = JsonObject({'aiOptIn': true}),
                            ),
                          );
                        });
                        unawaited(mixpanel.track('ai_opt_in', properties: {'enabled': true, 'via': 'ai_feedback'}));

                        if (context.mounted) {
                          await context.router.root.maybePop(true);
                        }
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
            if (isLoading.value) ...[
              Padding(
                padding: Pad(vertical: 16, bottom: bottomPadding + 12),
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
                      progress.value != null
                          ? progress.value!.phase == 'summarizing'
                                ? '분석 중... (${progress.value!.current}/${progress.value!.total})'
                                : '피드백 중... (${progress.value!.current}/${progress.value!.total})'
                          : '준비 중...',
                      style: TextStyle(fontSize: 13, color: context.colors.textFaint),
                    ),
                  ],
                ),
              ),
            ] else if (feedbacks.value.isEmpty) ...[
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
                    children: feedbacks.value
                        .map(
                          (feedback) => _AiFeedbackItem(
                            feedback: feedback,
                            isActive: activeFeedbackId.value == feedback.id,
                            onTap: () => onFeedbackTap(feedback),
                            onDismiss: () => onDismissFeedback(feedback),
                          ),
                        )
                        .toList(),
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

  final AiFeedback feedback;
  final bool isActive;
  final VoidCallback onTap;
  final VoidCallback onDismiss;

  @override
  Widget build(BuildContext context) {
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
                    feedback.startText == feedback.endText
                        ? '"${feedback.startText}"'
                        : '"${feedback.startText}" ... "${feedback.endText}"',
                    style: TextStyle(fontSize: 14, color: context.colors.textDefault),
                  ),
                  Text(
                    feedback.feedback,
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
