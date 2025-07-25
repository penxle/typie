import 'dart:async';

import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:gap/gap.dart';
import 'package:mixpanel_flutter/mixpanel_flutter.dart';
import 'package:typie/context/bottom_sheet.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/context/toast.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/screens/editor/limit.dart';
import 'package:typie/screens/editor/scope.dart';
import 'package:typie/widgets/tappable.dart';
import 'package:typie/widgets/webview.dart';

class SpellCheckBottomSheet extends HookWidget {
  const SpellCheckBottomSheet({required this.scope, required this.mixpanel, super.key});

  final EditorStateScope scope;
  final Mixpanel mixpanel;

  @override
  Widget build(BuildContext context) {
    final isLoading = useState(false);
    final errors = useState<List<Map<String, dynamic>>>([]);
    final webViewController = useValueListenable(scope.webViewController);

    useEffect(() {
      if (webViewController == null || isLoading.value) {
        return null;
      }

      isLoading.value = true;

      final completer = Completer<void>();
      StreamSubscription<WebViewEvent>? subscription;

      subscription = webViewController.onEvent.listen((event) async {
        if (event.name == 'spellcheckResult') {
          final data = event.data as Map<String, dynamic>;

          if (data['success'] == false) {
            errors.value = [];
            isLoading.value = false;
            unawaited(subscription?.cancel());
            completer.complete();
            if (data['needPlanUpgrade'] == true) {
              if (context.mounted) {
                // LimitBottomSheet를 먼저 열고 SpellCheckBottomSheet를 닫음
                await context.showBottomSheet(
                  intercept: true,
                  child: const LimitBottomSheet(type: LimitBottomSheetType.spellCheck),
                );
                await context.router.root.maybePop();
              }
            } else {
              if (context.mounted) {
                context.toast(ToastType.error, '맞춤법 검사에 실패했습니다');
                await context.router.root.maybePop();
              }
            }
          } else {
            // 정상적인 맞춤법 검사 결과
            if (data['errors'] != null) {
              errors.value = List<Map<String, dynamic>>.from(data['errors'] as List);
            } else {
              errors.value = [];
            }

            isLoading.value = false;
            unawaited(subscription?.cancel());
            completer.complete();

            unawaited(mixpanel.track('spellcheck', properties: {'errors': errors.value.length}));
          }
        }
      });

      // 맞춤법 검사 요청
      unawaited(
        webViewController.emitEvent('runSpellcheck', <String, dynamic>{}).then((_) {
          // 타임아웃 설정
          unawaited(
            Future<void>.delayed(const Duration(seconds: 10)).then((_) {
              if (!completer.isCompleted) {
                errors.value = [];
                isLoading.value = false;
                unawaited(subscription?.cancel());
                completer.completeError(Exception('Spellcheck timeout'));
              }
            }),
          );
        }),
      );

      return () {
        if (!completer.isCompleted) {
          completer.complete();
        }
        unawaited(subscription?.cancel());
      };
    }, [webViewController]);

    final mediaQuery = MediaQuery.of(context);
    final bottomPadding = mediaQuery.padding.bottom;

    return AppBottomSheet(
      includeBottomPadding: false,
      padding: const Pad(horizontal: 20),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Row(
            spacing: 8,
            children: [
              Icon(LucideLightIcons.spell_check, size: 20, color: context.colors.textSubtle),
              Text(
                '맞춤법 검사',
                style: TextStyle(fontSize: 18, fontWeight: FontWeight.w700, color: context.colors.textSubtle),
              ),
            ],
          ),
          const Gap(16),
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
              constraints: BoxConstraints(maxHeight: MediaQuery.of(context).size.height * 0.4),
              child: SingleChildScrollView(
                padding: Pad(bottom: bottomPadding + 12),
                child: Column(
                  children: errors.value
                      .map(
                        (error) => SpellCheckErrorItem(
                          error: error,
                          onCorrect: (correction) async {
                            // WebView에 수정 적용 이벤트 전송
                            final webViewController = scope.webViewController.value;
                            if (webViewController != null) {
                              await webViewController.emitEvent('applySpellCorrection', {
                                'from': error['from'],
                                'to': error['to'],
                                'correction': correction,
                              });

                              // 수정된 에러를 목록에서 제거
                              errors.value = errors.value
                                  .where((e) => !(e['from'] == error['from'] && e['to'] == error['to']))
                                  .toList();
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

class SpellCheckErrorItem extends StatelessWidget {
  const SpellCheckErrorItem({required this.error, required this.onCorrect, super.key});

  final Map<String, dynamic> error;
  final void Function(String) onCorrect;

  @override
  Widget build(BuildContext context) {
    return Container(
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
                      Text(
                        correction.toString(),
                        style: TextStyle(fontSize: 13, fontWeight: FontWeight.w600, color: context.colors.textDanger),
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
    );
  }
}

class SpellCheckErrorBottomSheet extends StatelessWidget {
  const SpellCheckErrorBottomSheet({required this.error, required this.onCorrect, super.key});

  final Map<String, dynamic> error;
  final void Function(String correction) onCorrect;

  @override
  Widget build(BuildContext context) {
    return AppBottomSheet(
      padding: const Pad(horizontal: 20),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Row(
            spacing: 8,
            children: [
              Icon(LucideLightIcons.spell_check, size: 20, color: context.colors.textSubtle),
              Text(
                '맞춤법 검사',
                style: TextStyle(fontSize: 18, fontWeight: FontWeight.w700, color: context.colors.textSubtle),
              ),
            ],
          ),
          const Gap(16),
          SpellCheckErrorItem(error: error, onCorrect: onCorrect),
        ],
      ),
    );
  }
}

void useSpellCheckErrorHandler(BuildContext context, EditorStateScope scope) {
  final webViewController = useValueListenable(scope.webViewController);

  useEffect(() {
    if (webViewController == null) {
      return null;
    }

    final subscription = webViewController.onEvent.listen((event) async {
      if (event.name == 'spellcheckErrorClick') {
        final error = event.data as Map<String, dynamic>;

        if (!context.mounted) {
          return;
        }

        await context.showBottomSheet(
          intercept: true,
          child: SpellCheckErrorBottomSheet(
            error: error,
            onCorrect: (correction) async {
              await webViewController.emitEvent('applySpellCorrection', {
                'from': error['from'],
                'to': error['to'],
                'correction': correction,
              });
              if (context.mounted) {
                await context.router.root.maybePop();
              }
            },
          ),
        );
      }
    });

    return subscription.cancel;
  }, [webViewController]);
}
