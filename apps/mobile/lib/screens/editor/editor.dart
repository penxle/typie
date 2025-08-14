import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:gap/gap.dart';
import 'package:mixpanel_flutter/mixpanel_flutter.dart';
import 'package:typie/context/bottom_sheet.dart';
import 'package:typie/context/modal.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/env.dart';
import 'package:typie/extensions/jiffy.dart';
import 'package:typie/extensions/num.dart';
import 'package:typie/graphql/__generated__/schema.schema.gql.dart';
import 'package:typie/graphql/widget.dart';
import 'package:typie/hooks/async_effect.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/icons/lucide_lab.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/modals/share.dart';
import 'package:typie/routers/app.gr.dart';
import 'package:typie/screens/editor/__generated__/delete_post_mutation.req.gql.dart';
import 'package:typie/screens/editor/__generated__/duplicate_post_mutation.req.gql.dart';
import 'package:typie/screens/editor/__generated__/editor_query.data.gql.dart';
import 'package:typie/screens/editor/__generated__/editor_query.req.gql.dart';
import 'package:typie/screens/editor/__generated__/update_post_type_mutation.req.gql.dart';
import 'package:typie/screens/editor/anchor.dart';
import 'package:typie/screens/editor/find_replace.dart';
import 'package:typie/screens/editor/limit.dart';
import 'package:typie/screens/editor/schema.dart';
import 'package:typie/screens/editor/scope.dart';
import 'package:typie/screens/editor/spellcheck.dart';
import 'package:typie/screens/editor/toolbar/floating/floating.dart';
import 'package:typie/screens/editor/toolbar/toolbar.dart';
import 'package:typie/services/auth.dart';
import 'package:typie/services/keyboard.dart';
import 'package:typie/services/preference.dart';
import 'package:typie/services/state.dart';
import 'package:typie/services/theme.dart';
import 'package:typie/widgets/forms/form.dart';
import 'package:typie/widgets/forms/select.dart';
import 'package:typie/widgets/heading.dart';
import 'package:typie/widgets/horizontal_divider.dart';
import 'package:typie/widgets/screen.dart';
import 'package:typie/widgets/tappable.dart';
import 'package:typie/widgets/webview.dart';
import 'package:url_launcher/url_launcher.dart';

class Editor extends HookWidget {
  const Editor({required this.slug, required this.goToNote, super.key});

  final String slug;
  final void Function() goToNote;

  @override
  Widget build(BuildContext context) {
    useAutomaticKeepAlive();

    final auth = useService<Auth>();
    final keyboard = useService<Keyboard>();
    final pref = useService<Pref>();
    final state = useService<AppState>();
    final theme = useService<AppTheme>();
    final mixpanel = useService<Mixpanel>();

    final isReady = useState(false);

    final scope = EditorStateScope.of(context);
    final webViewController = useValueListenable(scope.webViewController);
    final mode = useValueListenable(scope.mode);

    useEffect(() {
      final subscription = keyboard.onHeightChange.listen((height) {
        if (height > 0) {
          scope.keyboardHeight.value = height;
          scope.bottomToolbarMode.value = BottomToolbarMode.hidden;
        }

        scope.isKeyboardVisible.value = height > 0;
      });

      return subscription.cancel;
    }, []);

    useEffect(() {
      final subscription = keyboard.onTypeChange.listen((type) {
        scope.keyboardType.value = type;
      });

      return subscription.cancel;
    }, []);

    useAsyncEffect(() async {
      if (mode == EditorMode.editor && scope.isKeyboardVisible.value) {
        await Future<void>.delayed(const Duration(milliseconds: 500));
        await webViewController?.requestFocus();
      }

      return null;
    }, [mode]);

    useSpellCheckErrorHandler(context, scope);

    useEffect(() {
      if (webViewController == null) {
        return null;
      }

      final subscription = webViewController.onEvent.listen((event) async {
        switch (event.name) {
          case 'webviewReady':
            await webViewController.requestFocus();
            await webViewController.emitEvent('appReady', {
              'features': ['template', 'hide-table-delete-in-handle'],
              'settings': {
                'lineHighlightEnabled': pref.lineHighlightEnabled,
                'typewriterEnabled': pref.typewriterEnabled,
                'typewriterPosition': pref.typewriterPosition,
              },
              'state': {'selection': jsonDecode(state.getSerializedPostSelection(slug) ?? 'null')},
            });

            isReady.value = true;
          case 'setProseMirrorState':
            scope.proseMirrorState.value = ProseMirrorState.fromJson(event.data as Map<String, dynamic>);
          case 'setCharacterCountState':
            scope.characterCountState.value = CharacterCountState.fromJson(event.data as Map<String, dynamic>);
          case 'setYJSState':
            scope.yjsState.value = YJSState.fromJson(event.data as Map<String, dynamic>);
          case 'limitExceeded':
            await webViewController.clearFocus();
            if (context.mounted) {
              await context.showBottomSheet(intercept: true, child: const LimitBottomSheet());
            }
          case 'useTemplate':
            if (context.mounted) {
              await context.showBottomSheet(intercept: true, child: _TemplateBottomSheet(scope: scope));
            }
          case 'focus':
            final element = (event.data as Map<String, dynamic>)['element'] as String;
            scope.focusedElement.value = element;
            if (element == 'title' || element == 'subtitle') {
              unawaited(
                state.setSerializedPostSelection(slug, jsonEncode({'type': 'element', 'element': element})).catchError((
                  Object e,
                ) {
                  if (e is FileSystemException && e.osError?.errorCode == 28) {
                    // NOTE: 용량 부족 시 무시, 포커스 저장 안 함
                    return null;
                  }
                  return Future<void>.error(e);
                }),
              );
            }
          case 'blur':
            scope.focusedElement.value = null;
        }
      });

      return subscription.cancel;
    }, [webViewController]);

    useOnListenableChange(scope.proseMirrorState, () {
      if (!isReady.value) {
        return;
      }

      final selection = scope.proseMirrorState.value?.selection;
      if (selection == null) {
        return;
      }

      unawaited(
        state.setSerializedPostSelection(slug, jsonEncode(selection.toJson())).catchError((Object e) {
          if (e is FileSystemException && e.osError?.errorCode == 28) {
            // NOTE: 용량 부족 시 무시, 포커스 저장 안 함
            return null;
          }
          return Future<void>.error(e);
        }),
      );
    });

    return GraphQLOperation(
      initialBackgroundColor: context.colors.surfaceDefault,
      operation: GEditorScreen_QueryReq(
        (b) => b
          ..vars.slug = slug
          ..vars.siteId = pref.siteId,
      ),
      onLoaded: (data) {
        scope.data.value = data;
      },
      builder: (context, client, data) {
        return Screen(
          heading: Heading(
            titleIcon: data.post.type == GPostType.NORMAL ? LucideLabIcons.text_square : LucideLightIcons.shapes,
            title: data.post.title,
            actions: [
              HeadingAction(
                icon: LucideLightIcons.ellipsis,
                onTap: () async {
                  unawaited(scope.webViewController.value?.clearFocus());

                  var anchorCount = 0;

                  try {
                    final anchors = await scope.webViewController.value?.callProcedure('getAnchorPositionsV2');
                    if (anchors != null) {
                      anchorCount = (anchors as List<dynamic>).length;
                    }
                  } catch (_) {}

                  if (!context.mounted) {
                    return;
                  }

                  await context.showBottomSheet(
                    intercept: true,
                    child: AppBottomSheet(
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.stretch,
                        children: [
                          BottomMenuItem(
                            icon: LucideLightIcons.search,
                            label: '찾기',
                            onTap: () async {
                              await context.showBottomSheet(
                                intercept: true,
                                overlayOpacity: 0.05,
                                child: FindReplaceBottomSheet(scope: scope),
                              );
                            },
                          ),
                          BottomMenuItem(
                            icon: LucideLightIcons.bookmark,
                            label: '북마크',
                            trailing: anchorCount > 0
                                ? Container(
                                    decoration: BoxDecoration(
                                      border: Border.all(color: context.colors.borderStrong),
                                      borderRadius: BorderRadius.circular(4),
                                    ),
                                    padding: const Pad(horizontal: 8, vertical: 4),
                                    child: Text(
                                      '$anchorCount',
                                      style: TextStyle(
                                        fontSize: 12,
                                        fontWeight: FontWeight.w500,
                                        color: context.colors.textDefault,
                                      ),
                                    ),
                                  )
                                : null,
                            onTap: () async {
                              await context.showBottomSheet(
                                intercept: true,
                                overlayOpacity: 0.05,
                                child: AnchorBottomSheet(scope: scope),
                              );
                            },
                          ),
                          BottomMenuItem(
                            icon: LucideLightIcons.spell_check,
                            label: '맞춤법 검사',
                            onTap: () async {
                              if (data.me!.subscription == null) {
                                await context.showBottomSheet(
                                  intercept: true,
                                  child: const LimitBottomSheet(type: LimitBottomSheetType.spellCheck),
                                );

                                return;
                              }

                              unawaited(mixpanel.track('spellcheck', properties: {'via': 'editor'}));

                              await context.showBottomSheet(
                                intercept: true,
                                overlayOpacity: 0.05,
                                child: SpellCheckBottomSheet(scope: scope, mixpanel: mixpanel),
                              );
                            },
                          ),
                          const Gap(16),
                          HorizontalDivider(color: context.colors.borderDefault),
                          const Gap(16),
                          BottomMenuItem(
                            icon: LucideLightIcons.info,
                            label: '정보',
                            onTap: () async {
                              unawaited(mixpanel.track('open_post_info_modal', properties: {'via': 'editor'}));

                              await context.showBottomSheet(
                                intercept: true,
                                child: _EditorInfoBottomSheet(
                                  characterCountState: scope.characterCountState,
                                  post: data.post,
                                ),
                              );
                            },
                          ),
                          BottomMenuItem(icon: LucideLightIcons.notebook_tabs, label: '작성 노트', onTap: goToNote),
                          BottomMenuItem(
                            icon: LucideLightIcons.settings,
                            label: '본문 설정',
                            onTap: () async {
                              await context.showBottomSheet(intercept: true, child: _SettingBottomSheet(scope: scope));
                            },
                          ),
                          BottomMenuItem(
                            icon: LucideLightIcons.external_link,
                            label: '사이트에서 열기',
                            onTap: () async {
                              unawaited(mixpanel.track('open_post_in_browser', properties: {'via': 'editor'}));

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
                                      border: Border.all(color: context.colors.borderStrong),
                                      borderRadius: BorderRadius.circular(4),
                                    ),
                                    padding: const Pad(horizontal: 8, vertical: 4),
                                    child: Text(
                                      '링크 공개 중',
                                      style: TextStyle(
                                        fontSize: 12,
                                        fontWeight: FontWeight.w500,
                                        color: context.colors.textDefault,
                                      ),
                                    ),
                                  )
                                : null,
                            onTap: () async {
                              unawaited(mixpanel.track('open_post_share_modal', properties: {'via': 'editor'}));
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

                              unawaited(mixpanel.track('duplicate_post', properties: {'via': 'editor'}));

                              if (context.mounted) {
                                await context.router.popAndPush(EditorRoute(slug: res.duplicatePost.entity.slug));
                              }
                            },
                          ),
                          switch (data.post.type) {
                            GPostType.NORMAL => BottomMenuItem(
                              icon: LucideLightIcons.shapes,
                              label: '템플릿으로 전환',
                              onTap: () async {
                                await context.showModal(
                                  child: ConfirmModal(
                                    title: '템플릿으로 전환',
                                    message: '이 포스트를 템플릿으로 전환하시겠어요?\n앞으로 새 포스트를 생성할 때 이 포스트의 서식을 쉽게 이용할 수 있어요.',
                                    confirmText: '전환',
                                    onConfirm: () async {
                                      await client.request(
                                        GEditorScreen_UpdatePostType_MutationReq(
                                          (b) => b
                                            ..vars.input.postId = data.post.id
                                            ..vars.input.type = GPostType.TEMPLATE,
                                        ),
                                      );
                                    },
                                  ),
                                );
                              },
                            ),
                            GPostType.TEMPLATE => BottomMenuItem(
                              icon: LucideLightIcons.shapes,
                              label: '포스트로 전환',
                              onTap: () async {
                                await context.showModal(
                                  child: ConfirmModal(
                                    title: '포스트로 전환',
                                    message: '이 템플릿을 다시 일반 포스트로 전환하시겠어요?',
                                    confirmText: '전환',
                                    onConfirm: () async {
                                      await client.request(
                                        GEditorScreen_UpdatePostType_MutationReq(
                                          (b) => b
                                            ..vars.input.postId = data.post.id
                                            ..vars.input.type = GPostType.NORMAL,
                                        ),
                                      );
                                    },
                                  ),
                                );
                              },
                            ),
                            _ => throw UnimplementedError(),
                          },
                          BottomMenuItem(
                            icon: LucideLightIcons.trash_2,
                            label: '삭제하기',
                            onTap: () async {
                              await context.showModal(
                                intercept: true,
                                child: ConfirmModal(
                                  title: '포스트 삭제',
                                  message: '"${data.post.title}" 포스트를 삭제하시겠어요? 삭제 후 30일 동안 휴지통에 보관돼요.',
                                  confirmText: '삭제하기',
                                  confirmTextColor: context.colors.textBright,
                                  confirmBackgroundColor: context.colors.accentDanger,
                                  onConfirm: () async {
                                    await client.request(
                                      GEditorScreen_DeletePost_MutationReq((b) => b..vars.input.postId = data.post.id),
                                    );

                                    unawaited(mixpanel.track('delete_post', properties: {'via': 'editor'}));

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
                    ),
                  );
                },
              ),
            ],
            backgroundColor: context.colors.surfaceDefault,
          ),
          backgroundColor: context.colors.surfaceDefault,
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
                            initialUrl:
                                '${Env.websiteUrl}/_webview/editor?siteId=${data.post.entity.site.id}&slug=${data.post.entity.slug}',
                            initialCookies: [
                              Cookie('typie-at', (auth.value as Authenticated).accessToken),
                              Cookie('typie-th', switch (theme.mode) {
                                ThemeMode.system => 'auto',
                                ThemeMode.light => 'light',
                                ThemeMode.dark => 'dark',
                              }),
                            ],
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

class _SettingBottomSheet extends HookWidget {
  const _SettingBottomSheet({required this.scope});

  final EditorStateScope scope;

  @override
  Widget build(BuildContext context) {
    final yjsState = useValueListenable(scope.yjsState);
    final proseMirrorState = useValueListenable(scope.proseMirrorState);

    return AppBottomSheet(
      padding: const Pad(horizontal: 20),
      child: HookForm(
        submitMode: HookFormSubmitMode.onChange,
        onSubmit: (form) async {
          await scope.command('max_width', attrs: {'maxWidth': form.data['maxWidth']});
          await scope.command('body', attrs: {'paragraphIndent': form.data['paragraphIndent']});
          await scope.command('body', attrs: {'blockGap': form.data['blockGap']});
        },
        builder: (context, form) {
          return Column(
            spacing: 16,
            children: [
              _Option(
                icon: LucideLightIcons.ruler_dimension_line,
                label: '본문 폭',
                trailing: HookFormSelect(
                  name: 'maxWidth',
                  initialValue: yjsState?.maxWidth ?? 800,
                  items: const [
                    HookFormSelectItem(label: '600px', value: 600),
                    HookFormSelectItem(label: '800px', value: 800),
                    HookFormSelectItem(label: '1000px', value: 1000),
                  ],
                ),
              ),
              _Option(
                icon: LucideLightIcons.arrow_right_to_line,
                label: '첫 줄 들여쓰기',
                trailing: HookFormSelect(
                  name: 'paragraphIndent',
                  initialValue: (proseMirrorState?.nodes.isNotEmpty ?? false)
                      ? (proseMirrorState!.nodes.first.attrs?['paragraphIndent'] ?? 1)
                      : 1,
                  items: const [
                    HookFormSelectItem(label: '없음', value: 0),
                    HookFormSelectItem(label: '0.5칸', value: 0.5),
                    HookFormSelectItem(label: '1칸', value: 1),
                    HookFormSelectItem(label: '2칸', value: 2),
                  ],
                ),
              ),
              _Option(
                icon: LucideLightIcons.align_vertical_space_around,
                label: '문단 사이 간격',
                trailing: HookFormSelect(
                  name: 'blockGap',
                  initialValue: (proseMirrorState?.nodes.isNotEmpty ?? false)
                      ? (proseMirrorState!.nodes.first.attrs?['blockGap'] ?? 1)
                      : 1,
                  items: const [
                    HookFormSelectItem(label: '없음', value: 0),
                    HookFormSelectItem(label: '0.5줄', value: 0.5),
                    HookFormSelectItem(label: '1줄', value: 1),
                    HookFormSelectItem(label: '2줄', value: 2),
                  ],
                ),
              ),
            ],
          );
        },
      ),
    );
  }
}

class _Option extends StatelessWidget {
  const _Option({required this.icon, required this.label, required this.trailing});

  final IconData icon;
  final String label;
  final Widget trailing;

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      height: 24,
      child: Row(
        children: [
          Icon(icon, size: 20, color: context.colors.textSubtle),
          const Gap(8),
          Expanded(
            child: Text(label, style: TextStyle(fontSize: 16, color: context.colors.textSubtle)),
          ),
          trailing,
        ],
      ),
    );
  }
}

class _EditorInfoBottomSheet extends HookWidget {
  const _EditorInfoBottomSheet({required this.characterCountState, required this.post});

  final ValueNotifier<CharacterCountState?> characterCountState;
  final GEditorScreen_QueryData_post post;

  @override
  Widget build(BuildContext context) {
    final characterCountValue = useValueListenable(characterCountState);
    final difference = post.characterCountChange.additions - post.characterCountChange.deletions;

    return AppBottomSheet(
      padding: const Pad(horizontal: 20),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Text(
            '포스트 정보',
            style: TextStyle(fontSize: 17, fontWeight: FontWeight.w700, color: context.colors.textSubtle),
          ),
          const Gap(12),
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Text('최초 생성 시각', style: TextStyle(fontSize: 14, color: context.colors.textFaint)),
              Text(
                '${post.createdAt.toLocal().yyyyMMdd} ${post.createdAt.toLocal().Hm}',
                style: TextStyle(fontSize: 14, fontWeight: FontWeight.w500, color: context.colors.textSubtle),
              ),
            ],
          ),
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Text('마지막 수정 시각', style: TextStyle(fontSize: 14, color: context.colors.textFaint)),
              Text(
                '${post.updatedAt.toLocal().yyyyMMdd} ${post.updatedAt.toLocal().Hm}',
                style: TextStyle(fontSize: 14, fontWeight: FontWeight.w500, color: context.colors.textSubtle),
              ),
            ],
          ),
          const Gap(32),
          Text(
            '본문 정보',
            style: TextStyle(fontSize: 17, fontWeight: FontWeight.w700, color: context.colors.textSubtle),
          ),
          const Gap(12),
          Row(
            spacing: 4,
            children: [
              Icon(LucideLightIcons.type_, size: 15, color: context.colors.textSubtle),
              Text(
                '글자 수',
                style: TextStyle(fontSize: 15, fontWeight: FontWeight.w500, color: context.colors.textSubtle),
              ),
            ],
          ),
          const Gap(8),
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Text('공백 포함', style: TextStyle(fontSize: 14, color: context.colors.textFaint)),
              Text(
                '${characterCountValue?.countWithWhitespace.comma ?? '0'}자',
                style: TextStyle(fontSize: 14, fontWeight: FontWeight.w500, color: context.colors.textSubtle),
              ),
            ],
          ),
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Text('공백 미포함', style: TextStyle(fontSize: 14, color: context.colors.textFaint)),
              Text(
                '${characterCountValue?.countWithoutWhitespace.comma ?? '0'}자',
                style: TextStyle(fontSize: 14, fontWeight: FontWeight.w500, color: context.colors.textSubtle),
              ),
            ],
          ),
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Text('공백/부호 미포함', style: TextStyle(fontSize: 14, color: context.colors.textFaint)),
              Text(
                '${characterCountValue?.countWithoutWhitespaceAndPunctuation.comma ?? '0'}자',
                style: TextStyle(fontSize: 14, fontWeight: FontWeight.w500, color: context.colors.textSubtle),
              ),
            ],
          ),
          const Gap(12),
          Row(
            spacing: 4,
            children: [
              Icon(LucideLightIcons.goal, size: 15, color: context.colors.textSubtle),
              Text(
                '오늘의 기록',
                style: TextStyle(fontSize: 15, fontWeight: FontWeight.w500, color: context.colors.textSubtle),
              ),
            ],
          ),
          const Gap(8),
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Expanded(
                child: Text('변화량', style: TextStyle(fontSize: 14, color: context.colors.textFaint)),
              ),
              if (difference == 0)
                Text(
                  '없음',
                  style: TextStyle(fontSize: 14, fontWeight: FontWeight.w500, color: context.colors.textSubtle),
                )
              else ...[
                Icon(difference >= 0 ? LucideLightIcons.trending_up : LucideLightIcons.trending_down, size: 14),
                const Gap(4),
                Text(
                  '${difference >= 0 ? '+' : '-'}${difference.abs().comma}자',
                  style: TextStyle(fontSize: 14, fontWeight: FontWeight.w500, color: context.colors.textSubtle),
                ),
              ],
            ],
          ),
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Text('입력한 글자', style: TextStyle(fontSize: 14, color: context.colors.textFaint)),
              Text(
                '${post.characterCountChange.additions.comma}자',
                style: TextStyle(fontSize: 14, fontWeight: FontWeight.w500, color: context.colors.textSubtle),
              ),
            ],
          ),
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              Text('지운 글자', style: TextStyle(fontSize: 14, color: context.colors.textFaint)),
              Text(
                '${post.characterCountChange.deletions.comma}자',
                style: TextStyle(fontSize: 14, fontWeight: FontWeight.w500, color: context.colors.textSubtle),
              ),
            ],
          ),
        ],
      ),
    );
  }
}

class _TemplateBottomSheet extends HookWidget {
  const _TemplateBottomSheet({required this.scope});

  final EditorStateScope scope;

  @override
  Widget build(BuildContext context) {
    final data = useValueListenable(scope.data);
    final templates = data?.site.templates.toList() ?? [];

    return AppBottomSheet(
      child: templates.isEmpty
          ? Padding(
              padding: const Pad(vertical: 20),
              child: Text(
                '아직 템플릿이 없어요.\n\n에디터 우상단 더보기 메뉴에서\n기존 포스트를 템플릿으로 전환해보세요.',
                textAlign: TextAlign.center,
                style: TextStyle(fontSize: 14, color: context.colors.textFaint),
              ),
            )
          : ListView.separated(
              shrinkWrap: true,
              padding: const Pad(horizontal: 20),
              itemCount: templates.length,
              itemBuilder: (context, index) {
                return Tappable(
                  padding: const Pad(vertical: 8),
                  child: Row(
                    children: [
                      Expanded(child: Text(templates[index].title, overflow: TextOverflow.ellipsis)),
                      const Gap(8),
                      Text('사용하기', style: TextStyle(fontSize: 14, color: context.colors.textFaint)),
                      const Gap(4),
                      Icon(LucideLightIcons.chevron_right, size: 14, color: context.colors.textFaint),
                    ],
                  ),
                  onTap: () async {
                    await scope.webViewController.value?.emitEvent('loadTemplate', {
                      'slug': templates[index].entity.slug,
                    });
                    if (context.mounted) {
                      await context.router.root.maybePop();
                    }
                  },
                );
              },
              separatorBuilder: (context, index) {
                return const Gap(12);
              },
            ),
    );
  }
}
