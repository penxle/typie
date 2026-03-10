import 'dart:async';
import 'dart:math';

import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:auto_route/auto_route.dart';
import 'package:cached_network_image/cached_network_image.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:gap/gap.dart';
import 'package:gql_tristate_value/gql_tristate_value.dart';
import 'package:luthor/luthor.dart';
import 'package:mixpanel_flutter/mixpanel_flutter.dart';
import 'package:typie/constants/router_tab_index.dart';
import 'package:typie/context/bottom_sheet.dart';
import 'package:typie/context/modal.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/extensions/jiffy.dart';
import 'package:typie/extensions/num.dart';
import 'package:typie/graphql/__generated__/schema.schema.gql.dart';
import 'package:typie/graphql/client.dart';
import 'package:typie/graphql/widget.dart';
import 'package:typie/hooks/async_effect.dart';
import 'package:typie/hooks/route_resumed.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/icons/typie.dart';
import 'package:typie/modals/share.dart';
import 'package:typie/routers/app.gr.dart';
import 'package:typie/screens/entity/__generated__/create_document_mutation.req.gql.dart';
import 'package:typie/screens/entity/__generated__/create_folder_mutation.req.gql.dart';
import 'package:typie/screens/entity/__generated__/delete_document_mutation.req.gql.dart';
import 'package:typie/screens/entity/__generated__/delete_folder_mutation.req.gql.dart';
import 'package:typie/screens/entity/__generated__/duplicate_document_mutation.req.gql.dart';
import 'package:typie/screens/entity/__generated__/entity_fragment.data.gql.dart';
import 'package:typie/screens/entity/__generated__/move_entity_mutation.req.gql.dart';
import 'package:typie/screens/entity/__generated__/rename_folder_mutation.req.gql.dart';
import 'package:typie/screens/entity/__generated__/screen_with_entity_id_query.data.gql.dart';
import 'package:typie/screens/entity/__generated__/screen_with_entity_id_query.req.gql.dart';
import 'package:typie/screens/entity/__generated__/screen_with_site_id_query.req.gql.dart';
import 'package:typie/screens/entity/__generated__/site_fragment.data.gql.dart';
import 'package:typie/screens/entity/move_entity_modal.dart';
import 'package:typie/screens/entity/multi_entities_menu.dart';
import 'package:typie/screens/entity/selected_entities_bar.dart';
import 'package:typie/screens/entity/space_selector_bottom_sheet.dart';
import 'package:typie/services/site.dart';
import 'package:typie/widgets/forms/form.dart';
import 'package:typie/widgets/forms/text_field.dart';
import 'package:typie/widgets/haptic_reorderable.dart';
import 'package:typie/widgets/heading.dart';
import 'package:typie/widgets/screen.dart';
import 'package:typie/widgets/vertical_divider.dart';
import 'package:url_launcher/url_launcher.dart';

const maxDepth = 100;

@RoutePage()
class EntityRouter extends AutoRouter {
  const EntityRouter({super.key});
}

@RoutePage()
class EntityScreen extends StatelessWidget {
  const EntityScreen({super.key, @PathParam() this.entityId});

  final String? entityId;

  @override
  Widget build(BuildContext context) {
    return entityId == null ? const _WithSiteId() : _WithEntityId(entityId!);
  }
}

class _WithSiteId extends HookWidget {
  const _WithSiteId();

  @override
  Widget build(BuildContext context) {
    final site = useService<Site>();
    final siteId = useValueListenable(site);
    final refreshNotifier = useMemoized(RefreshNotifier.new, []);

    useRouteResumed(context, refreshNotifier.refresh, tabIndex: RouteTabsIndex.entity);

    return GraphQLOperation(
      initialBackgroundColor: context.colors.surfaceSubtle,
      operation: GEntityScreen_WithSiteId_QueryReq((b) => b..vars.siteId = siteId),
      refreshNotifier: refreshNotifier,
      builder: (context, client, data) {
        return _EntityList(null, data.site.entities.toList(), site: data.site, siteName: data.site.name);
      },
    );
  }
}

class _WithEntityId extends HookWidget {
  const _WithEntityId(this.entityId);

  final String entityId;

  @override
  Widget build(BuildContext context) {
    final refreshNotifier = useMemoized(RefreshNotifier.new, []);

    useRouteResumed(context, refreshNotifier.refresh, tabIndex: RouteTabsIndex.entity);

    return GraphQLOperation(
      initialBackgroundColor: context.colors.surfaceSubtle,
      operation: GEntityScreen_WithEntityId_QueryReq((b) => b..vars.entityId = entityId),
      refreshNotifier: refreshNotifier,
      builder: (context, client, data) {
        return _EntityList(data.entity, data.entity.children.toList(), siteName: data.entity.site.name);
      },
    );
  }
}

class _EntityList extends HookWidget {
  const _EntityList(this.entity, this.entities, {this.site, this.siteName});

  final GEntityScreen_WithEntityId_QueryData_entity? entity;
  final List<GEntityScreen_Entity_entity> entities;
  final GEntityScreen_Site_site? site;
  final String? siteName;

  GEntityScreen_WithEntityId_QueryData_entity_node__asFolder? get folder =>
      entity?.node as GEntityScreen_WithEntityId_QueryData_entity_node__asFolder?;

  @override
  Widget build(BuildContext context) {
    final client = useService<GraphQLClient>();
    final siteService = useService<Site>();
    final mixpanel = useService<Mixpanel>();

    final animationController = useAnimationController(duration: const Duration(milliseconds: 150));
    final textEditingController = useTextEditingController();
    final primaryScrollController = PrimaryScrollController.of(context);

    final isReordering = useState(false);
    final isRenaming = useState(false);
    final isSelecting = useState(false);
    final selectedItems = useState<Set<String>>({});
    final currentSiteLogoUrl = useState(site?.logo.url);
    final currentSiteName = useState(siteName);

    useEffect(() {
      void listener() {
        if (primaryScrollController.position.pixels > 0) {
          if (animationController.status != AnimationStatus.forward) {
            unawaited(animationController.forward());
          }
        } else {
          if (animationController.status != AnimationStatus.reverse) {
            unawaited(animationController.reverse());
          }
        }
      }

      primaryScrollController.addListener(listener);
      return () => primaryScrollController.removeListener(listener);
    }, [primaryScrollController]);

    useEffect(() {
      currentSiteLogoUrl.value = site?.logo.url;
      return null;
    }, [site?.logo.url]);

    useEffect(() {
      currentSiteName.value = siteName;
      return null;
    }, [siteName]);

    useAsyncEffect(() async {
      if (isRenaming.value) {
        textEditingController.selection = TextSelection(baseOffset: 0, extentOffset: textEditingController.text.length);
      }

      return null;
    }, [isRenaming.value]);

    return HookForm(
      schema: l.schema({'name': l.string().min(1).required()}),
      onSubmit: (form) async {
        await client.request(
          GEntityScreen_RenameFolder_MutationReq(
            (b) => b
              ..vars.input.folderId = folder!.id
              ..vars.input.name = form.data['name'] as String,
          ),
        );

        unawaited(mixpanel.track('rename_folder'));
        isRenaming.value = false;
      },
      builder: (context, form) {
        Future<void> showEntityMenu() async {
          if (isRenaming.value || isReordering.value || isSelecting.value) {
            return;
          }

          await context.showBottomSheet(
            child: BottomMenu(
              header: _BottomMenuHeader(
                entity: entity,
                siteName: currentSiteName.value,
                siteLogoUrl: currentSiteLogoUrl.value,
              ),
              items: [
                if (entity == null) ...[
                  BottomMenuItem(
                    icon: LucideLightIcons.settings,
                    label: '스페이스 설정',
                    onTap: () async {
                      unawaited(mixpanel.track('open_site_settings', properties: {'via': 'entity_menu'}));
                      await context.router.push(const SiteRoute());
                    },
                  ),
                  const BottomMenuSeparator(),
                ],
                if (entity != null) ...[
                  BottomMenuItem(
                    icon: LucideLightIcons.folder_symlink,
                    label: '다른 폴더로 옮기기',
                    onTap: () async {
                      unawaited(mixpanel.track('move_entity_try', properties: {'via': 'entity_menu'}));

                      await context.showBottomSheet(
                        intercept: true,
                        child: MoveEntityModal.single(entity: entity!, via: 'entity_menu'),
                      );
                    },
                  ),
                  BottomMenuItem(
                    icon: LucideLightIcons.external_link,
                    label: '스페이스에서 열기',
                    onTap: () async {
                      unawaited(mixpanel.track('open_folder_in_browser', properties: {'via': 'entity_menu'}));

                      final url = Uri.parse(entity!.url);
                      await launchUrl(url, mode: LaunchMode.externalApplication);
                    },
                  ),
                  BottomMenuItem(
                    icon: LucideLightIcons.blend,
                    label: '공유하기',
                    onTap: () async {
                      unawaited(mixpanel.track('open_folder_share_modal', properties: {'via': 'entity_menu'}));

                      await context.showBottomSheet(intercept: true, child: ShareBottomSheet(entityIds: [entity!.id]));
                    },
                  ),
                  const BottomMenuSeparator(),
                ],
                BottomMenuItem(
                  icon: LucideLightIcons.square_pen,
                  label: '여기에 문서 만들기',
                  onTap: () async {
                    final resp = await client.request(
                      GEntityScreen_CreateDocument_MutationReq(
                        (b) => b
                          ..vars.input.siteId = siteService.siteId
                          ..vars.input.parentEntityId = Value.present(entity?.id),
                      ),
                    );

                    unawaited(mixpanel.track('create_document', properties: {'via': 'entity_menu'}));

                    if (context.mounted) {
                      await context.router.push(NativeEditorRoute(slug: resp.createDocument.entity.slug));
                    }
                  },
                ),
                if ((entity?.depth ?? 0) < maxDepth - 1)
                  BottomMenuItem(
                    icon: LucideLightIcons.folder_plus,
                    label: '여기에 폴더 만들기',
                    onTap: () async {
                      final resp = await client.request(
                        GEntityScreen_CreateFolder_MutationReq(
                          (b) => b
                            ..vars.input.siteId = siteService.siteId
                            ..vars.input.parentEntityId = Value.present(entity?.id)
                            ..vars.input.name = '새 폴더',
                        ),
                      );

                      unawaited(mixpanel.track('create_folder'));

                      if (context.mounted) {
                        await context.router.push(EntityRoute(entityId: resp.createFolder.entity.id));
                      }
                    },
                  ),
                const BottomMenuSeparator(),
                if (entities.isNotEmpty) ...[
                  BottomMenuItem(
                    icon: LucideLightIcons.square_check,
                    label: '여러 항목 선택하기',
                    onTap: () {
                      isSelecting.value = true;
                      selectedItems.value = {};
                    },
                  ),
                  BottomMenuItem(
                    icon: LucideLightIcons.chevrons_up_down,
                    label: '순서 변경하기',
                    onTap: () {
                      isReordering.value = true;
                    },
                  ),
                  const BottomMenuSeparator(),
                ],
                if (entity != null) ...[
                  BottomMenuItem(
                    icon: LucideLightIcons.pen_line,
                    label: '이름 바꾸기',
                    onTap: () {
                      isRenaming.value = true;
                    },
                  ),
                  BottomMenuItem(
                    icon: LucideLightIcons.trash_2,
                    label: '삭제하기',
                    onTap: () async {
                      await context.showModal(
                        child: ConfirmModal(
                          title: '폴더 삭제',
                          message: '"${folder!.name}" 폴더를 삭제하시겠어요? 삭제 후 30일 동안 휴지통에 보관돼요.',
                          confirmText: '삭제하기',
                          confirmTextColor: context.colors.textBright,
                          confirmBackgroundColor: context.colors.accentDanger,
                          onConfirm: () async {
                            await client.request(
                              GEntityScreen_DeleteFolder_MutationReq((b) => b..vars.input.folderId = folder!.id),
                            );

                            unawaited(mixpanel.track('delete_folder'));

                            if (context.mounted) {
                              await context.router.maybePop();
                            }
                          },
                        ),
                      );
                    },
                  ),
                ],
                if (entity == null) ...[
                  BottomMenuItem(
                    icon: LucideLightIcons.trash_2,
                    label: '휴지통',
                    onTap: () async {
                      await context.router.push(TrashRoute());
                    },
                  ),
                ],
              ],
            ),
          );
        }

        return Screen(
          heading: Heading(
            titleWidget: Row(
              spacing: 8,
              children: [
                if (entity != null) const Icon(LucideLightIcons.folder, size: 20),
                Expanded(
                  child: isRenaming.value
                      ? HookFormTextField.collapsed(
                          name: 'name',
                          controller: textEditingController,
                          autofocus: true,
                          initialValue: folder!.name,
                          placeholder: '폴더 이름',
                          style: const TextStyle(fontSize: 18, fontWeight: FontWeight.w600),
                        )
                      : entity == null
                      ? GestureDetector(
                          behavior: HitTestBehavior.opaque,
                          onTap: () async {
                            await context.showBottomSheet(
                              child: SpaceSelectorBottomSheet(
                                onSiteChanged: () {
                                  final tabsRouter = AutoTabsRouter.of(context);
                                  final entityRouter = tabsRouter.childControllers.first;
                                  if (entityRouter is StackRouter) {
                                    entityRouter.popUntilRoot();
                                  }
                                },
                                onUpgrade: () {
                                  unawaited(context.router.push(const EnrollPlanRoute()));
                                },
                              ),
                            );
                          },
                          child: Row(
                            spacing: 8,
                            children: [
                              if (currentSiteLogoUrl.value != null)
                                ClipRRect(
                                  borderRadius: BorderRadius.circular(4),
                                  child: CachedNetworkImage(
                                    imageUrl:
                                        '${currentSiteLogoUrl.value}?s=${pow(2, (log(24 * MediaQuery.devicePixelRatioOf(context)) / log(2)).ceil()).toInt()}&q=75',
                                    width: 24,
                                    height: 24,
                                    fit: BoxFit.cover,
                                  ),
                                )
                              else
                                const Icon(LucideLightIcons.folder_open, size: 20),
                              Expanded(
                                child: Text(
                                  currentSiteName.value ?? '내 스페이스',
                                  style: const TextStyle(fontSize: 18, fontWeight: FontWeight.w600),
                                  overflow: TextOverflow.ellipsis,
                                ),
                              ),
                              Icon(LucideLightIcons.chevron_down, size: 14, color: context.colors.textSubtle),
                            ],
                          ),
                        )
                      : GestureDetector(
                          onDoubleTap: () {
                            isRenaming.value = true;
                          },
                          child: Text(
                            textEditingController.text.isEmpty ? folder!.name : textEditingController.text,
                            style: const TextStyle(fontSize: 18, fontWeight: FontWeight.w600),
                            overflow: TextOverflow.ellipsis,
                          ),
                        ),
                ),
              ],
            ),
            leadingWidget: isRenaming.value
                ? HeadingLeading(
                    icon: LucideLightIcons.x,
                    onTap: () {
                      isRenaming.value = false;
                      textEditingController.text = '';
                    },
                  )
                : null,
            actions: [
              if (!isRenaming.value && !isReordering.value && !isSelecting.value)
                HeadingAction(icon: LucideLightIcons.ellipsis, onTap: showEntityMenu)
              else if (isSelecting.value)
                HeadingAction(
                  icon: LucideLightIcons.x,
                  onTap: () {
                    isSelecting.value = false;
                    selectedItems.value = {};
                  },
                )
              else
                HeadingAction(
                  icon: LucideLightIcons.check,
                  onTap: () async {
                    if (isRenaming.value) {
                      await form.submit();
                    } else if (isReordering.value) {
                      isReordering.value = false;
                    }
                  },
                ),
            ],
          ),
          child: GestureDetector(
            behavior: HitTestBehavior.translucent,
            onLongPress: showEntityMenu,
            child: Stack(
              children: [
                if (entities.isEmpty)
                  Center(
                    child: Text(
                      entity == null ? '스페이스가 비어있어요' : '폴더가 비어있어요',
                      style: TextStyle(fontSize: 16, fontWeight: FontWeight.w500, color: context.colors.textFaint),
                    ),
                  )
                else
                  HapticReorderableList(
                    orderedIds: [for (final item in entities) item.id],
                    controller: primaryScrollController,
                    physics: const AlwaysScrollableScrollPhysics(),
                    padding: Pad(horizontal: 20, top: 14, bottom: isSelecting.value ? 90 : 14),
                    itemBuilder: (context, index) {
                      return Padding(
                        padding: const Pad(vertical: 6),
                        child: GestureDetector(
                          onTap: () {
                            if (isReordering.value) {
                              return;
                            }

                            if (isSelecting.value) {
                              final currentSelection = Set<String>.from(selectedItems.value);
                              if (currentSelection.contains(entities[index].id)) {
                                currentSelection.remove(entities[index].id);
                              } else {
                                currentSelection.add(entities[index].id);
                              }
                              selectedItems.value = currentSelection;
                              return;
                            }

                            unawaited(
                              entities[index].node.when(
                                folder: (folder) => context.router.push(EntityRoute(entityId: entities[index].id)),
                                document: (document) =>
                                    context.router.push(NativeEditorRoute(slug: entities[index].slug)),
                                orElse: () => throw UnimplementedError(),
                              ),
                            );
                          },
                          onLongPress: () async {
                            if (isReordering.value) {
                              return;
                            }

                            if (isSelecting.value && selectedItems.value.contains(entities[index].id)) {
                              await context.showBottomSheet(
                                child: MultiEntitiesMenu(
                                  selectedItems: selectedItems.value,
                                  entities: entities,
                                  onExitSelectionMode: () {
                                    isSelecting.value = false;
                                    selectedItems.value = {};
                                  },
                                  via: 'entity_long_press',
                                ),
                              );
                              return;
                            }

                            await entities[index].node.when(
                              folder: (folder) => context.showBottomSheet(
                                child: BottomMenu(
                                  header: _BottomMenuHeader(entity: entities[index], siteName: currentSiteName.value),
                                  items: [
                                    BottomMenuItem(
                                      icon: LucideLightIcons.folder_symlink,
                                      label: '다른 폴더로 옮기기',
                                      onTap: () async {
                                        unawaited(
                                          mixpanel.track('move_entity_try', properties: {'via': 'entity_folder_menu'}),
                                        );

                                        await context.showBottomSheet(
                                          intercept: true,
                                          child: MoveEntityModal.single(
                                            entity: entities[index],
                                            via: 'entity_folder_menu',
                                          ),
                                        );
                                      },
                                    ),
                                    BottomMenuItem(
                                      icon: LucideLightIcons.external_link,
                                      label: '스페이스에서 열기',
                                      onTap: () async {
                                        unawaited(
                                          mixpanel.track(
                                            'open_folder_in_browser',
                                            properties: {'via': 'entity_folder_menu'},
                                          ),
                                        );

                                        final url = Uri.parse(entities[index].url);
                                        await launchUrl(url, mode: LaunchMode.externalApplication);
                                      },
                                    ),
                                    BottomMenuItem(
                                      icon: LucideLightIcons.blend,
                                      label: '공유하기',
                                      onTap: () async {
                                        unawaited(
                                          mixpanel.track(
                                            'open_folder_share_modal',
                                            properties: {'via': 'entity_folder_menu'},
                                          ),
                                        );

                                        await context.showBottomSheet(
                                          intercept: true,
                                          child: ShareBottomSheet(entityIds: [entities[index].id]),
                                        );
                                      },
                                    ),
                                    const BottomMenuSeparator(),
                                    BottomMenuItem(
                                      icon: LucideLightIcons.square_pen,
                                      label: '하위 문서 만들기',
                                      onTap: () async {
                                        final resp = await client.request(
                                          GEntityScreen_CreateDocument_MutationReq(
                                            (b) => b
                                              ..vars.input.siteId = siteService.siteId
                                              ..vars.input.parentEntityId = Value.present(entities[index].id),
                                          ),
                                        );

                                        unawaited(
                                          mixpanel.track('create_document', properties: {'via': 'entity_folder_menu'}),
                                        );

                                        if (context.mounted) {
                                          await context.router.push(
                                            NativeEditorRoute(slug: resp.createDocument.entity.slug),
                                          );
                                        }
                                      },
                                    ),
                                    if (entities[index].depth < maxDepth - 1)
                                      BottomMenuItem(
                                        icon: LucideLightIcons.folder_plus,
                                        label: '하위 폴더 만들기',
                                        onTap: () async {
                                          final resp = await client.request(
                                            GEntityScreen_CreateFolder_MutationReq(
                                              (b) => b
                                                ..vars.input.siteId = siteService.siteId
                                                ..vars.input.parentEntityId = Value.present(entities[index].id)
                                                ..vars.input.name = '새 폴더',
                                            ),
                                          );

                                          unawaited(
                                            mixpanel.track('create_folder', properties: {'via': 'entity_folder_menu'}),
                                          );

                                          if (context.mounted) {
                                            await context.router.push(
                                              EntityRoute(entityId: resp.createFolder.entity.id),
                                            );
                                          }
                                        },
                                      ),
                                    const BottomMenuSeparator(),
                                    if (!isSelecting.value && !isReordering.value) ...[
                                      BottomMenuItem(
                                        icon: LucideLightIcons.square_check,
                                        label: '여러 항목 선택하기',
                                        onTap: () {
                                          isSelecting.value = true;
                                          selectedItems.value = {entities[index].id};
                                        },
                                      ),
                                      BottomMenuItem(
                                        icon: LucideLightIcons.chevrons_up_down,
                                        label: '순서 변경하기',
                                        onTap: () {
                                          isReordering.value = true;
                                        },
                                      ),
                                      const BottomMenuSeparator(),
                                    ],
                                    BottomMenuItem(
                                      icon: LucideLightIcons.trash_2,
                                      label: '삭제하기',
                                      onTap: () async {
                                        await context.showModal(
                                          child: ConfirmModal(
                                            title: '폴더 삭제',
                                            message: '"${folder.name}" 폴더를 삭제하시겠어요? 삭제 후 30일 동안 휴지통에 보관돼요.',
                                            confirmText: '삭제하기',
                                            confirmTextColor: context.colors.textBright,
                                            confirmBackgroundColor: context.colors.accentDanger,
                                            onConfirm: () async {
                                              await client.request(
                                                GEntityScreen_DeleteFolder_MutationReq(
                                                  (b) => b..vars.input.folderId = folder.id,
                                                ),
                                              );

                                              unawaited(
                                                mixpanel.track(
                                                  'delete_folder',
                                                  properties: {'via': 'entity_folder_menu'},
                                                ),
                                              );
                                            },
                                          ),
                                        );
                                      },
                                    ),
                                  ],
                                ),
                              ),
                              document: (document) => context.showBottomSheet(
                                child: BottomMenu(
                                  header: _BottomMenuHeader(entity: entities[index], siteName: currentSiteName.value),
                                  items: [
                                    BottomMenuItem(
                                      icon: LucideLightIcons.file_symlink,
                                      label: '다른 폴더로 옮기기',
                                      onTap: () async {
                                        unawaited(
                                          mixpanel.track(
                                            'move_entity_try',
                                            properties: {'via': 'entity_document_menu'},
                                          ),
                                        );

                                        await context.showBottomSheet(
                                          intercept: true,
                                          child: MoveEntityModal.single(
                                            entity: entities[index],
                                            via: 'entity_document_menu',
                                          ),
                                        );
                                      },
                                    ),
                                    BottomMenuItem(
                                      icon: LucideLightIcons.external_link,
                                      label: '스페이스에서 열기',
                                      onTap: () async {
                                        unawaited(
                                          mixpanel.track(
                                            'open_document_in_browser',
                                            properties: {'via': 'entity_document_menu'},
                                          ),
                                        );

                                        final url = Uri.parse(entities[index].url);
                                        await launchUrl(url, mode: LaunchMode.externalApplication);
                                      },
                                    ),
                                    BottomMenuItem(
                                      icon: LucideLightIcons.blend,
                                      label: '공유하기',
                                      onTap: () async {
                                        unawaited(
                                          mixpanel.track(
                                            'open_document_share_modal',
                                            properties: {'via': 'entity_document_menu'},
                                          ),
                                        );

                                        await context.showBottomSheet(
                                          intercept: true,
                                          child: ShareBottomSheet(entityIds: [entities[index].id]),
                                        );
                                      },
                                    ),
                                    BottomMenuItem(
                                      icon: LucideLightIcons.copy,
                                      label: '복제하기',
                                      onTap: () async {
                                        await client.request(
                                          GEntityScreen_DuplicateDocument_MutationReq(
                                            (b) => b..vars.input.documentId = document.id,
                                          ),
                                        );

                                        unawaited(
                                          mixpanel.track(
                                            'duplicate_document',
                                            properties: {'via': 'entity_document_menu'},
                                          ),
                                        );
                                      },
                                    ),
                                    const BottomMenuSeparator(),
                                    if (!isSelecting.value && !isReordering.value) ...[
                                      BottomMenuItem(
                                        icon: LucideLightIcons.square_check,
                                        label: '여러 항목 선택하기',
                                        onTap: () {
                                          isSelecting.value = true;
                                          selectedItems.value = {entities[index].id};
                                        },
                                      ),
                                      BottomMenuItem(
                                        icon: LucideLightIcons.chevrons_up_down,
                                        label: '순서 변경하기',
                                        onTap: () {
                                          isReordering.value = true;
                                        },
                                      ),
                                      const BottomMenuSeparator(),
                                    ],
                                    BottomMenuItem(
                                      icon: LucideLightIcons.trash_2,
                                      label: '삭제하기',
                                      onTap: () async {
                                        await context.showModal(
                                          intercept: true,
                                          child: ConfirmModal(
                                            title: '문서 삭제',
                                            message: '"${document.title}" 문서를 삭제하시겠어요? 삭제 후 30일 동안 휴지통에 보관돼요.',
                                            confirmText: '삭제하기',
                                            confirmTextColor: context.colors.textBright,
                                            confirmBackgroundColor: context.colors.accentDanger,
                                            onConfirm: () async {
                                              await client.request(
                                                GEntityScreen_DeleteDocument_MutationReq(
                                                  (b) => b..vars.input.documentId = document.id,
                                                ),
                                              );

                                              unawaited(
                                                mixpanel.track(
                                                  'delete_document',
                                                  properties: {'via': 'entity_document_menu'},
                                                ),
                                              );
                                            },
                                          ),
                                        );
                                      },
                                    ),
                                  ],
                                ),
                              ),
                              orElse: () => throw UnimplementedError(),
                            );
                          },
                          child: IntrinsicHeight(
                            child: Container(
                              decoration: BoxDecoration(
                                border: Border.all(color: context.colors.borderStrong),
                                borderRadius: const BorderRadius.all(Radius.circular(8)),
                                color: isSelecting.value && selectedItems.value.contains(entities[index].id)
                                    ? context.colors.accentInfo.withValues(alpha: 0.1)
                                    : context.colors.surfaceDefault,
                              ),
                              child: Row(
                                crossAxisAlignment: CrossAxisAlignment.stretch,
                                children: [
                                  if (isReordering.value) ...[
                                    ReorderableDragStartListener(
                                      index: index,
                                      child: const Listener(
                                        behavior: HitTestBehavior.opaque,
                                        child: Padding(
                                          padding: Pad(all: 12),
                                          child: Icon(LucideLightIcons.grip_vertical, size: 20),
                                        ),
                                      ),
                                    ),
                                    AppVerticalDivider(color: context.colors.borderStrong),
                                  ] else if (isSelecting.value) ...[
                                    Listener(
                                      behavior: HitTestBehavior.opaque,
                                      child: Padding(
                                        padding: const Pad(all: 12),
                                        child: Icon(
                                          selectedItems.value.contains(entities[index].id)
                                              ? LucideLightIcons.square_check
                                              : LucideLightIcons.square,
                                          size: 20,
                                          color: selectedItems.value.contains(entities[index].id)
                                              ? context.colors.textDefault
                                              : context.colors.textSubtle,
                                        ),
                                      ),
                                    ),
                                    AppVerticalDivider(color: context.colors.borderStrong),
                                  ],
                                  const Gap(16),
                                  Expanded(
                                    child: Padding(
                                      padding: const Pad(vertical: 12),
                                      child: entities[index].node.when(
                                        folder: (_) => _Folder(entities[index]),
                                        document: (_) => _Document(entities[index]),
                                        orElse: () => throw UnimplementedError(),
                                      ),
                                    ),
                                  ),
                                  const Gap(16),
                                ],
                              ),
                            ),
                          ),
                        ),
                      );
                    },
                    proxyDecorator: (child, index, animation) => child,
                    onReorder: (oldIndex, newIndex) async {
                      var adjustedNewIndex = newIndex;
                      if (oldIndex < newIndex) {
                        adjustedNewIndex -= 1;
                      }
                      if (oldIndex == adjustedNewIndex) {
                        return;
                      }

                      final dragging = entities[oldIndex];
                      String? lowerOrder;
                      String? upperOrder;

                      if (newIndex >= entities.length) {
                        lowerOrder = entities[entities.length - 1].order;
                        entities
                          ..remove(dragging)
                          ..add(dragging);
                      } else if (newIndex == 0) {
                        upperOrder = entities[0].order;
                        entities
                          ..remove(dragging)
                          ..insert(newIndex, dragging);
                      } else {
                        lowerOrder = entities[newIndex - 1].order;
                        upperOrder = entities[newIndex].order;

                        if (oldIndex > newIndex) {
                          entities
                            ..removeAt(oldIndex)
                            ..insert(newIndex, dragging);
                        } else {
                          entities
                            ..remove(dragging)
                            ..insert(newIndex - 1, dragging);
                        }
                      }

                      await client.request(
                        GEntityScreen_MoveEntity_MutationReq(
                          (b) => b
                            ..vars.input.entityId = dragging.id
                            ..vars.input.parentEntityId = Value.present(entity?.id)
                            ..vars.input.lowerOrder = Value.present(lowerOrder)
                            ..vars.input.upperOrder = Value.present(upperOrder),
                        ),
                      );

                      unawaited(mixpanel.track('move_entity', properties: {'via': 'reorder'}));
                    },
                  ),
                if (isSelecting.value)
                  SelectedEntitiesBar(
                    isVisible: selectedItems.value.isNotEmpty,
                    selectedItems: selectedItems.value,
                    entities: entities,
                    onClearSelection: () {
                      selectedItems.value = {};
                    },
                    onExitSelectionMode: () {
                      isSelecting.value = false;
                      selectedItems.value = {};
                    },
                  ),
              ],
            ),
          ),
        );
      },
    );
  }
}

class _Folder extends StatelessWidget {
  // ignore: unused_element_parameter for future usage
  const _Folder(this.entity, {this.color});

  final GEntityScreen_Entity_entity entity;
  GEntityScreen_Entity_entity_node__asFolder get folder => entity.node as GEntityScreen_Entity_entity_node__asFolder;
  final Color? color;

  @override
  Widget build(BuildContext context) {
    return Row(
      spacing: 8,
      children: [
        Icon(TypieIcons.folder_filled, size: 18, color: color),
        Expanded(
          child: Text(
            folder.name,
            style: TextStyle(fontSize: 16, fontWeight: FontWeight.w500, color: color),
            overflow: TextOverflow.ellipsis,
            maxLines: 1,
          ),
        ),
        const Icon(LucideLightIcons.chevron_right, size: 16),
      ],
    );
  }
}

class _Document extends StatelessWidget {
  const _Document(this.entity);

  final GEntityScreen_Entity_entity entity;
  GEntityScreen_Entity_entity_node__asDocument get document =>
      entity.node as GEntityScreen_Entity_entity_node__asDocument;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      spacing: 4,
      children: [
        Row(
          spacing: 8,
          children: [
            Icon(
              document.documentType == GDocumentType.TEMPLATE
                  ? LucideLightIcons.layout_template
                  : LucideLightIcons.file,
              size: 18,
            ),
            Expanded(
              child: Text(
                document.title,
                style: const TextStyle(fontSize: 16, fontWeight: FontWeight.w500),
                overflow: TextOverflow.ellipsis,
                maxLines: 1,
              ),
            ),
            Text(document.updatedAt.ago, style: TextStyle(fontSize: 14, color: context.colors.textSubtle)),
          ],
        ),
        Text(
          document.excerpt.isEmpty ? '(내용 없음)' : document.excerpt,
          style: TextStyle(fontSize: 14, color: context.colors.textSubtle),
          overflow: TextOverflow.ellipsis,
          maxLines: 1,
        ),
      ],
    );
  }
}

class _BottomMenuHeader extends StatelessWidget {
  const _BottomMenuHeader({this.entity, this.siteName, this.siteLogoUrl});

  final GEntityScreen_Entity_entity? entity;
  final String? siteName;
  final String? siteLogoUrl;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      spacing: 4,
      children: [
        Row(
          spacing: 16,
          children: [
            if (entity == null && siteLogoUrl != null)
              ClipRRect(
                borderRadius: BorderRadius.circular(4),
                child: CachedNetworkImage(imageUrl: siteLogoUrl!, width: 20, height: 20, fit: BoxFit.cover),
              )
            else
              Icon(
                entity?.node.when(
                      folder: (_) => LucideLightIcons.folder,
                      document: (doc) => doc.documentType == GDocumentType.TEMPLATE
                          ? LucideLightIcons.layout_template
                          : LucideLightIcons.file,
                      orElse: () => throw UnimplementedError(),
                    ) ??
                    LucideLightIcons.folder_open,
                size: 20,
              ),
            Expanded(
              child: Text(
                entity?.node.when(
                      folder: (folder) => folder.name,
                      document: (document) => document.title,
                      orElse: () => throw UnimplementedError(),
                    ) ??
                    siteName ??
                    '내 스페이스',
                style: const TextStyle(fontSize: 17, fontWeight: FontWeight.w600),
                overflow: TextOverflow.ellipsis,
                maxLines: 1,
              ),
            ),
          ],
        ),
        if (entity != null)
          Padding(
            padding: const Pad(left: 36),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              spacing: 4,
              children: [
                LayoutBuilder(
                  builder: (context, constraints) {
                    final textStyle = TextStyle(fontSize: 14, color: context.colors.textSubtle);
                    final maxItemWidth = (constraints.maxWidth * 0.6).clamp(120.0, 220.0);

                    return Wrap(
                      spacing: 4,
                      runSpacing: 2,
                      crossAxisAlignment: WrapCrossAlignment.center,
                      children: [
                        _BottomMenuBreadcrumbLabel(
                          text: siteName ?? '내 스페이스',
                          style: textStyle,
                          maxWidth: maxItemWidth,
                        ),
                        ...entity!.ancestors.map(
                          (ancestor) => Row(
                            mainAxisSize: MainAxisSize.min,
                            children: [
                              const Icon(LucideLightIcons.chevron_right, size: 14),
                              const SizedBox(width: 4),
                              _BottomMenuBreadcrumbLabel(
                                text: ancestor.node.when(
                                  folder: (folder) => folder.name,
                                  orElse: () => throw UnimplementedError(),
                                ),
                                style: textStyle,
                                maxWidth: maxItemWidth,
                              ),
                            ],
                          ),
                        ),
                      ],
                    );
                  },
                ),
                if (entity!.node.when(folder: (folder) => true, document: (document) => true, orElse: () => false))
                  Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        entity!.visibility == GEntityVisibility.PUBLIC
                            ? '공개'
                            : entity!.visibility == GEntityVisibility.UNLISTED &&
                                  entity!.availability == GEntityAvailability.UNLISTED
                            ? '링크 조회/편집 가능'
                            : entity!.visibility == GEntityVisibility.UNLISTED
                            ? '링크 조회 가능'
                            : entity!.availability == GEntityAvailability.UNLISTED
                            ? '링크 편집 가능'
                            : '비공개',
                        style: TextStyle(
                          fontSize: 14,
                          color:
                              entity!.visibility == GEntityVisibility.PUBLIC ||
                                  entity!.visibility == GEntityVisibility.UNLISTED ||
                                  entity!.availability == GEntityAvailability.UNLISTED
                              ? context.colors.accentBrand
                              : context.colors.textFaint,
                        ),
                      ),
                      Text(
                        entity!.node.maybeWhen(
                          folder: (folder) {
                            final parts = <String>[];
                            if (folder.folderCount > 0) {
                              parts.add('폴더 ${folder.folderCount.comma}개');
                            }
                            if (folder.documentCount > 0) {
                              parts.add('문서 ${folder.documentCount.comma}개');
                            }
                            parts.add('총 ${folder.characterCount.comma}자');
                            return parts.join(' · ');
                          },
                          document: (document) => '총 ${document.characterCount.comma}자',
                          orElse: () => '',
                        ),
                        style: TextStyle(fontSize: 14, color: context.colors.textFaint),
                      ),
                    ],
                  ),
              ],
            ),
          ),
      ],
    );
  }
}

class _BottomMenuBreadcrumbLabel extends StatelessWidget {
  const _BottomMenuBreadcrumbLabel({required this.text, required this.style, required this.maxWidth});

  final String text;
  final TextStyle style;
  final double maxWidth;

  @override
  Widget build(BuildContext context) {
    return ConstrainedBox(
      constraints: BoxConstraints(maxWidth: maxWidth),
      child: Text(text, style: style, overflow: TextOverflow.ellipsis, maxLines: 1),
    );
  }
}
