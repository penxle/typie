import 'dart:async';

import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:gap/gap.dart';
import 'package:mixpanel_flutter/mixpanel_flutter.dart';
import 'package:sentry_flutter/sentry_flutter.dart';
import 'package:typie/context/bottom_sheet.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/context/toast.dart';
import 'package:typie/hooks/async_effect.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/screens/editor/scope.dart';
import 'package:typie/widgets/tappable.dart';

class SpellCheckBottomSheet extends HookWidget {
  const SpellCheckBottomSheet({required this.scope, required this.mixpanel, super.key});

  final EditorStateScope scope;
  final Mixpanel mixpanel;

  @override
  Widget build(BuildContext context) {
    final isLoading = useState(false);
    final errors = useState<List<Map<String, dynamic>>>([]);
    final webViewController = useValueListenable(scope.webViewController);

    useAsyncEffect(() async {
      if (webViewController == null || isLoading.value) {
        return null;
      }

      isLoading.value = true;

      try {
        final result = await webViewController.callProcedure('checkSpelling') as Map<String, dynamic>;
        errors.value = (result['errors'] as List).map((e) => e as Map<String, dynamic>).toList();
      } catch (err) {
        unawaited(Sentry.captureException(err));

        if (context.mounted) {
          context.toast(ToastType.error, '맞춤법 검사에 실패했습니다');
          await context.router.root.maybePop();
        }
      } finally {
        isLoading.value = false;
      }

      return null;
    }, [webViewController]);

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
              constraints: BoxConstraints(maxHeight: MediaQuery.of(context).size.height * 0.4),
              child: SingleChildScrollView(
                padding: Pad(bottom: bottomPadding + 12),
                child: Column(
                  children: errors.value
                      .map(
                        (error) => SpellCheckErrorItem(
                          error: error,
                          onCorrect: (correction) async {
                            if (scope.webViewController.value != null) {
                              await scope.webViewController.value!.callProcedure('applySpellcheckCorrection', {
                                'id': error['id'],
                                'correction': correction,
                              });

                              errors.value = errors.value.where((e) => e['id'] != error['id']).toList();
                            }
                          },
                          onTap: () async {
                            if (scope.webViewController.value != null) {
                              await scope.webViewController.value!.callProcedure('scrollToSpellcheckError', {
                                'id': error['id'],
                              });
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
  const SpellCheckErrorItem({required this.error, required this.onCorrect, required this.onTap, super.key});

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

class SpellCheckErrorBottomSheet extends StatelessWidget {
  const SpellCheckErrorBottomSheet({required this.error, required this.onCorrect, required this.onTap, super.key});

  final Map<String, dynamic> error;
  final void Function(String correction) onCorrect;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    return AppBottomSheet(
      padding: const Pad(horizontal: 20),
      child: SpellCheckErrorItem(error: error, onCorrect: onCorrect, onTap: onTap),
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
          overlayOpacity: 0.05,
          child: SpellCheckErrorBottomSheet(
            error: error,
            onCorrect: (correction) async {
              await webViewController.callProcedure('applySpellcheckCorrection', {
                'id': error['id'],
                'correction': correction,
              });

              if (context.mounted) {
                await context.router.root.maybePop();
              }
            },
            onTap: () async {
              await webViewController.callProcedure('scrollToSpellcheckError', {'id': error['id']});
            },
          ),
        );
      }
    });

    return subscription.cancel;
  }, [webViewController]);
}
