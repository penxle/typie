import 'dart:async';
import 'dart:io';

import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:gap/gap.dart';
import 'package:typie/context/bottom_sheet.dart';
import 'package:typie/context/modal.dart';
import 'package:typie/env.dart';
import 'package:typie/extensions/num.dart';
import 'package:typie/graphql/__generated__/schema.schema.gql.dart';
import 'package:typie/graphql/widget.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/icons/lucide_lab.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/modals/share.dart';
import 'package:typie/routers/app.gr.dart';
import 'package:typie/screens/editor/__generated__/delete_post_mutation.req.gql.dart';
import 'package:typie/screens/editor/__generated__/duplicate_post_mutation.req.gql.dart';
import 'package:typie/screens/editor/__generated__/editor_query.req.gql.dart';
import 'package:typie/screens/editor/schema.dart';
import 'package:typie/screens/editor/scope.dart';
import 'package:typie/screens/editor/toolbar/floating/floating.dart';
import 'package:typie/screens/editor/toolbar/toolbar.dart';
import 'package:typie/services/auth.dart';
import 'package:typie/services/keyboard.dart';
import 'package:typie/services/preference.dart';
import 'package:typie/styles/colors.dart';
import 'package:typie/widgets/heading.dart';
import 'package:typie/widgets/horizontal_divider.dart';
import 'package:typie/widgets/screen.dart';
import 'package:typie/widgets/tappable.dart';
import 'package:typie/widgets/webview.dart';
import 'package:url_launcher/url_launcher.dart';

class Editor extends HookWidget {
  const Editor({required this.slug, super.key});

  final String slug;

  @override
  Widget build(BuildContext context) {
    final auth = useService<Auth>();
    final keyboard = useService<Keyboard>();
    final pref = useService<Pref>();

    final isReady = useState(false);

    final scope = EditorStateScope.of(context);
    final webViewController = useValueListenable(scope.webViewController);

    useEffect(() {
      final subscription = keyboard.onHeightChange.listen((height) {
        if (height > 0) {
          scope.keyboardHeight.value = height;
          scope.bottomToolbarMode.value = BottomToolbarMode.hidden;
        }

        scope.isKeyboardVisible.value = height > 0;
      });

      return subscription.cancel;
    }, [keyboard.onHeightChange]);

    useEffect(() {
      if (webViewController == null) {
        return null;
      }

      final subscription = webViewController.onEvent.listen((event) async {
        switch (event.name) {
          case 'webviewReady':
            isReady.value = true;
            await webViewController.requestFocus();
            await webViewController.emitEvent('appReady');
          case 'setProseMirrorState':
            scope.proseMirrorState.value = ProseMirrorState.fromJson(event.data as Map<String, dynamic>);
          case 'setCharacterCountState':
            scope.characterCountState.value = CharacterCountState.fromJson(event.data as Map<String, dynamic>);
          case 'limitExceeded':
            await webViewController.clearFocus();
            if (context.mounted) {
              await context.showBottomSheet(intercept: true, child: const _LimitBottomSheet());
            }
        }
      });

      return subscription.cancel;
    }, [webViewController]);

    return GraphQLOperation(
      initialBackgroundColor: AppColors.white,
      operation: GEditorScreen_QueryReq((b) => b..vars.slug = slug),
      onLoaded: (data) {
        scope.data.value = data;
      },
      builder: (context, client, data) {
        return Screen(
          heading: Heading(
            titleIcon: LucideLabIcons.text_square,
            title: data.post.title,
            actions: [
              HeadingAction(
                icon: LucideLightIcons.ellipsis,
                onTap: () async {
                  unawaited(scope.webViewController.value?.clearFocus());
                  await context.showBottomSheet(
                    intercept: true,
                    child: BottomMenu(
                      items: [
                        BottomMenuItem(
                          icon: LucideLightIcons.info,
                          label: '정보',
                          onTap: () async {
                            await context.showBottomSheet(
                              intercept: true,
                              child: _EditorInfoBottomSheet(characterCountState: scope.characterCountState),
                            );
                          },
                        ),
                        BottomMenuItem(
                          icon: LucideLightIcons.external_link,
                          label: '사이트에서 열기',
                          onTap: () async {
                            final url = Uri.parse(data.post.entity.url);
                            await launchUrl(url, mode: LaunchMode.externalApplication);
                          },
                        ),
                        BottomMenuItem(
                          icon: LucideLightIcons.blend,
                          label: '공유하기',
                          trailing: data.post.entity.visibility == GEntityVisibility.UNLISTED
                              ? Container(
                                  decoration: BoxDecoration(
                                    border: Border.all(color: AppColors.gray_950),
                                    borderRadius: BorderRadius.circular(4),
                                  ),
                                  padding: const Pad(horizontal: 8, vertical: 4),
                                  child: const Text(
                                    '링크 공개 중',
                                    style: TextStyle(
                                      fontSize: 12,
                                      fontWeight: FontWeight.w500,
                                      color: AppColors.gray_950,
                                    ),
                                  ),
                                )
                              : null,
                          onTap: () async {
                            await context.showBottomSheet(intercept: true, child: SharePostBottomSheet(slug: slug));
                          },
                        ),
                        BottomMenuItem(
                          icon: LucideLightIcons.copy,
                          label: '복제하기',
                          onTap: () async {
                            final res = await client.request(
                              GEditorScreen_DuplicatePost_MutationReq((b) => b..vars.input.postId = data.post.id),
                            );

                            if (context.mounted) {
                              await context.router.popAndPush(EditorRoute(slug: res.duplicatePost.entity.slug));
                            }
                          },
                        ),
                        BottomMenuItem(
                          icon: LucideLightIcons.trash,
                          label: '삭제하기',
                          onTap: () async {
                            await context.showModal(
                              intercept: true,
                              child: ConfirmModal(
                                title: '포스트 삭제',
                                message: '"${data.post.title}" 포스트를 삭제하시겠어요?',
                                confirmText: '삭제하기',
                                confirmColor: AppColors.red_500,
                                onConfirm: () async {
                                  await client.request(
                                    GEditorScreen_DeletePost_MutationReq((b) => b..vars.input.postId = data.post.id),
                                  );

                                  if (context.mounted) {
                                    await context.router.maybePop();
                                  }
                                },
                              ),
                            );
                          },
                        ),
                      ],
                    ),
                  );
                },
              ),
            ],
            backgroundColor: AppColors.white,
          ),
          backgroundColor: AppColors.white,
          keyboardDismiss: false,
          responsive: false,
          child: Stack(
            fit: StackFit.expand,
            children: [
              Opacity(
                opacity: isReady.value ? 1 : 0.01,
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: [
                    Expanded(
                      child: Stack(
                        children: [
                          WebView(
                            initialUrl: '${Env.websiteUrl}/_webview/editor?siteId=${pref.siteId}&slug=$slug',
                            initialCookies: [Cookie('typie-at', (auth.value as Authenticated).accessToken)],
                            onWebViewCreated: (controller) {
                              scope.webViewController.value = controller;
                            },
                          ),
                          const Positioned(bottom: 20, right: 20, child: EditorFloatingToolbar()),
                        ],
                      ),
                    ),
                    const EditorToolbar(),
                  ],
                ),
              ),
              if (!isReady.value) const Positioned.fill(child: Center(child: CircularProgressIndicator())),
            ],
          ),
        );
      },
    );
  }
}

class _LimitBottomSheet extends StatelessWidget {
  const _LimitBottomSheet();

  @override
  Widget build(BuildContext context) {
    final List<IconData> icons = [
      LucideLightIcons.crown,
      LucideLightIcons.tag,
      LucideLightIcons.star,
      LucideLightIcons.key,
      LucideLightIcons.gift,
    ];

    return AppBottomSheet(
      padding: const Pad(horizontal: 20),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Center(
            child: SizedBox(
              height: 32,
              width: 32 + (icons.length - 1) * 22,
              child: Stack(
                children: [
                  for (int i = 0; i < icons.length; i++)
                    Positioned(
                      left: i * 22,
                      child: Container(
                        decoration: BoxDecoration(
                          color: AppColors.gray_950,
                          border: Border.all(color: AppColors.white, width: 2),
                          borderRadius: BorderRadius.circular(999),
                        ),
                        padding: const Pad(all: 6),
                        child: Icon(icons[i], size: 16, color: AppColors.white),
                      ),
                    ),
                ],
              ),
            ),
          ),
          const Gap(16),
          const Text(
            '플랜 업그레이드가 필요해요',
            textAlign: TextAlign.center,
            style: TextStyle(fontSize: 18, fontWeight: FontWeight.w600),
          ),
          const Gap(4),
          const Text(
            '현재 플랜의 최대 사용량을 초과했어요.',
            textAlign: TextAlign.center,
            style: TextStyle(fontSize: 14, color: AppColors.gray_500),
          ),
          const Text(
            '이어서 작성하려면 플랜을 업그레이드 해주세요.',
            textAlign: TextAlign.center,
            style: TextStyle(fontSize: 14, color: AppColors.gray_500),
          ),
          const Gap(16),
          Container(
            decoration: BoxDecoration(
              border: Border.all(color: AppColors.gray_950),
              borderRadius: BorderRadius.circular(8),
            ),
            child: const Padding(
              padding: Pad(all: 16),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  Text('타이피 FULL ACCESS', style: TextStyle(fontSize: 16, fontWeight: FontWeight.w600)),
                  Gap(12),
                  HorizontalDivider(color: AppColors.gray_950),
                  Gap(12),
                  Column(
                    spacing: 8,
                    children: [
                      _FeatureItem(icon: LucideLightIcons.book_open_text, label: '무제한 글자 수'),
                      _FeatureItem(icon: LucideLightIcons.images, label: '무제한 파일 업로드'),
                      _FeatureItem(icon: LucideLightIcons.link, label: '커스텀 공유 주소'),
                      _FeatureItem(icon: LucideLightIcons.flask_conical, label: '베타 기능 우선 접근'),
                      _FeatureItem(icon: LucideLightIcons.headset, label: '문제 발생 시 우선 지원'),
                      _FeatureItem(icon: LucideLightIcons.sprout, label: '디스코드 커뮤니티 참여'),
                      _FeatureItem(icon: LucideLightIcons.ellipsis, label: '그리고 더 많은 혜택'),
                    ],
                  ),
                ],
              ),
            ),
          ),
          const Gap(16),
          Tappable(
            onTap: () async {
              await context.router.root.maybePop();
              if (context.mounted) {
                await context.router.popAndPush(const EnrollPlanRoute());
              }
            },
            child: Container(
              decoration: BoxDecoration(color: AppColors.gray_950, borderRadius: BorderRadius.circular(8)),
              padding: const Pad(vertical: 16),
              child: const Text(
                '업그레이드',
                style: TextStyle(fontSize: 16, fontWeight: FontWeight.w700, color: AppColors.white),
                textAlign: TextAlign.center,
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class _FeatureItem extends StatelessWidget {
  const _FeatureItem({required this.icon, required this.label});

  final IconData icon;
  final String label;

  @override
  Widget build(BuildContext context) {
    return Row(
      spacing: 8,
      children: [
        Icon(icon, size: 16),
        Text(label, style: const TextStyle(fontSize: 14)),
      ],
    );
  }
}

class _EditorInfoBottomSheet extends HookWidget {
  const _EditorInfoBottomSheet({required this.characterCountState});

  final ValueNotifier<CharacterCountState?> characterCountState;

  @override
  Widget build(BuildContext context) {
    final characterCountValue = useValueListenable(characterCountState);

    return AppBottomSheet(
      padding: const Pad(horizontal: 20),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          const Text(
            '본문 정보',
            style: TextStyle(fontSize: 16, fontWeight: FontWeight.w700, color: AppColors.gray_700),
          ),
          const Gap(12),
          const Text(
            '글자 수',
            style: TextStyle(fontSize: 14, fontWeight: FontWeight.w500, color: AppColors.gray_700),
          ),
          const Gap(8),
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              const Text('공백 포함', style: TextStyle(fontSize: 14, color: AppColors.gray_500)),
              Text(
                '${characterCountValue?.countWithWhitespace.comma ?? '0'}자',
                style: const TextStyle(fontSize: 14, fontWeight: FontWeight.w500, color: AppColors.gray_700),
              ),
            ],
          ),
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              const Text('공백 미포함', style: TextStyle(fontSize: 14, color: AppColors.gray_500)),
              Text(
                '${characterCountValue?.countWithoutWhitespace.comma ?? '0'}자',
                style: const TextStyle(fontSize: 14, fontWeight: FontWeight.w500, color: AppColors.gray_700),
              ),
            ],
          ),
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              const Text('공백/부호 미포함', style: TextStyle(fontSize: 14, color: AppColors.gray_500)),
              Text(
                '${characterCountValue?.countWithoutWhitespaceAndPunctuation.comma ?? '0'}자',
                style: const TextStyle(fontSize: 14, fontWeight: FontWeight.w500, color: AppColors.gray_700),
              ),
            ],
          ),
        ],
      ),
    );
  }
}
