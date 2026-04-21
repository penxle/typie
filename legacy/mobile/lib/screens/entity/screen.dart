import 'dart:async';
import 'dart:math' as math;
import 'dart:ui' as ui;

import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:gap/gap.dart';
import 'package:gql_tristate_value/gql_tristate_value.dart';
import 'package:mixpanel_flutter/mixpanel_flutter.dart';
import 'package:typie/context/bottom_sheet.dart';
import 'package:typie/context/modal.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/context/toast.dart';
import 'package:typie/extensions/jiffy.dart';
import 'package:typie/extensions/num.dart';
import 'package:typie/graphql/__generated__/schema.schema.gql.dart';
import 'package:typie/graphql/client.dart';
import 'package:typie/graphql/widget.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/icons/lucide_light.dart';
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
import 'package:typie/screens/native_editor/auto_discard.dart';
import 'package:typie/screens/shell/nav.dart';
import 'package:typie/services/site.dart';
import 'package:typie/widgets/haptic_reorderable.dart';
import 'package:typie/widgets/heading.dart';
import 'package:typie/widgets/overlay_heading.dart';
import 'package:typie/widgets/popover/list.dart';
import 'package:typie/widgets/popover/popover.dart';
import 'package:typie/widgets/screen.dart';
import 'package:typie/widgets/space_popover_button.dart';
import 'package:typie/widgets/tappable.dart';
import 'package:typie/widgets/vertical_divider.dart';
import 'package:url_launcher/url_launcher.dart';

const maxDepth = 100;
const _headingPopoverScreenPadding = EdgeInsets.fromLTRB(20, 8, 20, 8);
const _entityHeadingFadeHeight = OverlayHeading.height + OverlayHeading.gradientHeight;
const _entityListBottomPadding = 140.0;
const _entitySelectionBarBottomOffset = 20.0;
const _entityShellNavHeight = 60.0;
const _entityShellNavBottomGap = 12.0;
const _entitySelectionBarHeight = 56.0;
const _entitySelectionBarContentGap = 24.0;
const _entityReorderEdgeFadeHeight = 16.0;
const _entityPaneHeaderTopHeight = 28.0;
const _entityPaneHeaderIconTargetLeft = 16.0;
const _entityPaneHeaderIconTargetSize = 20.0;
const _entityPaneHeaderTitleGap = 12.0;
const _entityPaneHeaderTextRightInset = 16.0;
const _entityPaneHeaderSourceHorizontalInset = 14.0;
const _entityPaneHeaderSourceIconSize = 18.0;
const _entityPaneHeaderSourceIconGap = 10.0;

List<BoxShadow> _headingControlShadow(BuildContext context) => [
  BoxShadow(color: context.colors.shadowDefault.withValues(alpha: 0.06), offset: const Offset(0, 1), blurRadius: 4),
];

class _RectDragBoundaryDelegate extends DragBoundaryDelegate<Rect> {
  _RectDragBoundaryDelegate(this.boundary);

  final Rect boundary;

  @override
  bool isWithinBoundary(Rect draggedObject) {
    return boundary.contains(draggedObject.topLeft) && boundary.contains(draggedObject.bottomRight);
  }

  @override
  Rect nearestPositionWithinBoundary(Rect draggedObject) {
    final maxLeft = boundary.right - draggedObject.width;
    final maxTop = boundary.bottom - draggedObject.height;

    if (maxLeft < boundary.left || maxTop < boundary.top) {
      throw FlutterError('The dragged rect is larger than the available boundary.');
    }

    return Rect.fromLTWH(
      draggedObject.left.clamp(boundary.left, maxLeft),
      draggedObject.top.clamp(boundary.top, maxTop),
      draggedObject.width,
      draggedObject.height,
    );
  }
}

class _EntityReorderEdgeFade extends StatelessWidget {
  const _EntityReorderEdgeFade({required this.begin, required this.end, required this.baseColor});

  final Alignment begin;
  final Alignment end;
  final Color baseColor;

  @override
  Widget build(BuildContext context) {
    return IgnorePointer(
      child: SizedBox(
        height: _entityReorderEdgeFadeHeight,
        child: DecoratedBox(
          decoration: BoxDecoration(
            gradient: LinearGradient(begin: begin, end: end, colors: [baseColor, baseColor.withValues(alpha: 0)]),
          ),
        ),
      ),
    );
  }
}

String _buildSpaceSummary(List<GEntityScreen_Entity_entity> entities) {
  var folderCount = 0;
  var documentCount = 0;

  for (final entity in entities) {
    entity.node.when(
      folder: (_) => folderCount += 1,
      document: (_) => documentCount += 1,
      orElse: () => throw UnimplementedError(),
    );
  }

  final parts = <String>[
    if (folderCount > 0) '폴더 ${folderCount.comma}개',
    if (documentCount > 0) '문서 ${documentCount.comma}개',
  ];

  return parts.isEmpty ? '비어 있는 스페이스' : parts.join(' · ');
}

String _buildFolderSummary(GEntityScreen_WithEntityId_QueryData_entity_node__asFolder folder) {
  final parts = <String>[
    if (folder.folderCount > 0) '폴더 ${folder.folderCount.comma}개',
    if (folder.documentCount > 0) '문서 ${folder.documentCount.comma}개',
  ];

  return parts.isEmpty ? '빈 폴더' : parts.join(' · ');
}

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

    return _EntityTrailingActionScope(
      child: GraphQLOperation(
        initialBackgroundColor: context.colors.surfaceSubtle,
        operation: GEntityScreen_WithSiteId_QueryReq((b) => b..vars.siteId = siteId),
        builder: (context, client, data) {
          return _EntityList(null, data.site.entities.toList(), site: data.site, siteName: data.site.name);
        },
      ),
    );
  }
}

class _WithEntityId extends StatelessWidget {
  const _WithEntityId(this.entityId);

  final String entityId;

  @override
  Widget build(BuildContext context) {
    return _EntityTrailingActionScope(
      parentEntityId: entityId,
      child: GraphQLOperation(
        initialBackgroundColor: context.colors.surfaceSubtle,
        operation: GEntityScreen_WithEntityId_QueryReq((b) => b..vars.entityId = entityId),
        builder: (context, client, data) {
          final child = _EntityList(data.entity, data.entity.children.toList(), siteName: data.entity.site.name);
          if (data.entity.depth < maxDepth - 1) {
            return child;
          }

          return _EntityTrailingActionScope(parentEntityId: entityId, canCreateFolder: false, child: child);
        },
      ),
    );
  }
}

class _EntityTrailingActionScope extends HookWidget {
  const _EntityTrailingActionScope({required this.child, this.parentEntityId, this.canCreateFolder = true});

  final Widget child;
  final String? parentEntityId;
  final bool canCreateFolder;

  @override
  Widget build(BuildContext context) {
    final client = useService<GraphQLClient>();
    final siteService = useService<Site>();
    final mixpanel = useService<Mixpanel>();
    final trailingActionController = ShellNav.maybeOf(context)?.trailingAction;
    final trailingActionOwner = useMemoized(Object.new);

    Future<void> createDocumentHere() async {
      final resp = await client.request(
        GEntityScreen_CreateDocument_MutationReq(
          (b) => b
            ..vars.input.siteId = siteService.siteId
            ..vars.input.parentEntityId = Value.present(parentEntityId),
        ),
      );

      unawaited(mixpanel.track('create_document', properties: {'via': 'entity_menu'}));

      if (context.mounted) {
        markAutoDiscardCandidate(resp.createDocument.entity.slug);
        await context.router.push(NativeEditorRoute(slug: resp.createDocument.entity.slug));
      }
    }

    Future<void> createFolderHere() async {
      final resp = await client.request(
        GEntityScreen_CreateFolder_MutationReq(
          (b) => b
            ..vars.input.siteId = siteService.siteId
            ..vars.input.parentEntityId = Value.present(parentEntityId)
            ..vars.input.name = '새 폴더',
        ),
      );

      unawaited(mixpanel.track('create_folder'));

      if (context.mounted) {
        await context.router.push(EntityRoute(entityId: resp.createFolder.entity.id));
      }
    }

    useEffect(() {
      trailingActionController?.setFor(
        trailingActionOwner,
        ShellTrailingActionConfig(
          icon: LucideLightIcons.square_plus,
          pane: _EntityCreatePopoverPane(
            onCreateDocument: createDocumentHere,
            onCreateFolder: canCreateFolder ? createFolderHere : null,
          ),
        ),
      );

      return () {
        trailingActionController?.clearFor(trailingActionOwner);
      };
    }, [canCreateFolder, parentEntityId, trailingActionController, trailingActionOwner]);

    return child;
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

    final primaryScrollController = PrimaryScrollController.of(context);
    final topInset = MediaQuery.paddingOf(context).top;
    final shellNav = ShellNav.maybeOf(context);
    final fallbackShellNavVisible = useMemoized(() => ValueNotifier(false));
    final shellNavVisible = useValueListenable(shellNav?.visible ?? fallbackShellNavVisible);
    final reorderBoundaryKey = useMemoized(GlobalKey.new);

    final isReordering = useState(false);
    final isSelecting = useState(false);
    final selectedItems = useState<Set<String>>({});
    final currentSiteLogoUrl = useState(site?.logo.url);
    final currentSiteName = useState(siteName);

    useEffect(() {
      return fallbackShellNavVisible.dispose;
    }, [fallbackShellNavVisible]);

    useEffect(() {
      currentSiteLogoUrl.value = site?.logo.url;
      return null;
    }, [site?.logo.url]);

    useEffect(() {
      currentSiteName.value = siteName;
      return null;
    }, [siteName]);

    Future<void> openSiteSettings() async {
      unawaited(mixpanel.track('open_site_settings', properties: {'via': 'entity_menu'}));
      await context.router.push(const SiteSettingsRoute());
    }

    Future<void> moveCurrentEntity() async {
      if (entity == null) {
        return;
      }

      unawaited(mixpanel.track('move_entity_try', properties: {'via': 'entity_menu'}));

      await context.showBottomSheet(
        intercept: true,
        child: MoveEntityModal.single(entity: entity!, via: 'entity_menu'),
      );
    }

    Future<void> openCurrentEntityInSpace() async {
      if (entity == null) {
        return;
      }

      unawaited(mixpanel.track('open_folder_in_browser', properties: {'via': 'entity_menu'}));

      final url = Uri.parse(entity!.url);
      await launchUrl(url, mode: LaunchMode.externalApplication);
    }

    Future<void> openCurrentEntityShare() async {
      if (entity == null) {
        return;
      }

      unawaited(mixpanel.track('open_folder_share_modal', properties: {'via': 'entity_menu'}));
      await context.showBottomSheet(intercept: true, child: ShareBottomSheet(entityIds: [entity!.id]));
    }

    Future<void> renameCurrentEntity() async {
      if (folder == null) {
        return;
      }

      await context.showBottomSheet(
        child: _EntityRenameBottomSheet(
          client: client,
          mixpanel: mixpanel,
          folderId: folder!.id,
          initialName: folder!.name,
        ),
      );
    }

    Future<void> deleteCurrentEntity() async {
      if (folder == null) {
        return;
      }

      await context.showModal(
        child: ConfirmModal(
          title: '폴더 삭제',
          message: '"${folder!.name}" 폴더를 삭제하시겠어요? 삭제 후 30일 동안 휴지통에 보관돼요.',
          confirmText: '삭제하기',
          confirmTextColor: context.colors.textBright,
          confirmBackgroundColor: context.colors.accentDanger,
          onConfirm: () async {
            await client.request(GEntityScreen_DeleteFolder_MutationReq((b) => b..vars.input.folderId = folder!.id));

            unawaited(mixpanel.track('delete_folder'));

            if (context.mounted) {
              await context.router.maybePop();
            }
          },
        ),
      );
    }

    void startSelectionMode() {
      isSelecting.value = true;
      selectedItems.value = {};
    }

    void exitSelectionMode() {
      isSelecting.value = false;
      selectedItems.value = {};
    }

    Widget buildListRow({
      required int index,
      required Color backgroundColor,
      required bool enableDragHandle,
      required bool showDivider,
    }) {
      final isSelected = selectedItems.value.contains(entities[index].id);
      final borderRadius = BorderRadius.vertical(
        top: Radius.circular(index == 0 ? 12 : 0),
        bottom: Radius.circular(showDivider ? 0 : 12),
      );

      Widget buildLeading() {
        if (isReordering.value) {
          const Widget handle = Listener(
            behavior: HitTestBehavior.opaque,
            child: Padding(padding: Pad(all: 12), child: Icon(LucideLightIcons.grip_vertical, size: 20)),
          );

          if (!enableDragHandle) {
            return handle;
          }

          return ReorderableDragStartListener(index: index, child: handle);
        }

        if (isSelecting.value) {
          return Listener(
            behavior: HitTestBehavior.opaque,
            child: Padding(
              padding: const Pad(all: 12),
              child: Icon(
                isSelected ? LucideLightIcons.square_check : LucideLightIcons.square,
                size: 20,
                color: isSelected ? context.colors.textDefault : context.colors.textSubtle,
              ),
            ),
          );
        }

        return const SizedBox.shrink();
      }

      return Container(
        decoration: BoxDecoration(
          color: backgroundColor,
          borderRadius: borderRadius,
          border: showDivider ? Border(bottom: BorderSide(color: context.colors.borderSubtle)) : null,
        ),
        child: IntrinsicHeight(
          child: Row(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              if (isReordering.value || isSelecting.value) ...[
                buildLeading(),
                AppVerticalDivider(color: context.colors.borderSubtle),
              ],
              Expanded(
                child: Padding(
                  padding: const Pad(horizontal: 16, vertical: 14),
                  child: entities[index].node.when(
                    folder: (_) => _Folder(entities[index]),
                    document: (_) => _Document(entities[index]),
                    orElse: () => throw UnimplementedError(),
                  ),
                ),
              ),
            ],
          ),
        ),
      );
    }

    void handleEntityTap(int index) {
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
          document: (document) => context.router.push(NativeEditorRoute(slug: entities[index].slug)),
          orElse: () => throw UnimplementedError(),
        ),
      );
    }

    Future<void> handleEntityLongPress(int index) async {
      final isSelected = selectedItems.value.contains(entities[index].id);

      if (isReordering.value) {
        return;
      }

      if (isSelecting.value && isSelected) {
        await context.showBottomSheet(
          child: MultiEntitiesMenu(
            selectedItems: selectedItems.value,
            entities: entities,
            onExitSelectionMode: exitSelectionMode,
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
                  unawaited(mixpanel.track('move_entity_try', properties: {'via': 'entity_folder_menu'}));

                  await context.showBottomSheet(
                    intercept: true,
                    child: MoveEntityModal.single(entity: entities[index], via: 'entity_folder_menu'),
                  );
                },
              ),
              BottomMenuItem(
                icon: LucideLightIcons.external_link,
                label: '스페이스에서 열기',
                onTap: () async {
                  unawaited(mixpanel.track('open_folder_in_browser', properties: {'via': 'entity_folder_menu'}));

                  final url = Uri.parse(entities[index].url);
                  await launchUrl(url, mode: LaunchMode.externalApplication);
                },
              ),
              BottomMenuItem(
                icon: LucideLightIcons.blend,
                label: '공유하기',
                onTap: () async {
                  unawaited(mixpanel.track('open_folder_share_modal', properties: {'via': 'entity_folder_menu'}));

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

                  unawaited(mixpanel.track('create_document', properties: {'via': 'entity_folder_menu'}));

                  if (context.mounted) {
                    markAutoDiscardCandidate(resp.createDocument.entity.slug);
                    await context.router.push(NativeEditorRoute(slug: resp.createDocument.entity.slug));
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

                    unawaited(mixpanel.track('create_folder', properties: {'via': 'entity_folder_menu'}));

                    if (context.mounted) {
                      await context.router.push(EntityRoute(entityId: resp.createFolder.entity.id));
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
                          GEntityScreen_DeleteFolder_MutationReq((b) => b..vars.input.folderId = folder.id),
                        );

                        unawaited(mixpanel.track('delete_folder', properties: {'via': 'entity_folder_menu'}));
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
                  unawaited(mixpanel.track('move_entity_try', properties: {'via': 'entity_document_menu'}));

                  await context.showBottomSheet(
                    intercept: true,
                    child: MoveEntityModal.single(entity: entities[index], via: 'entity_document_menu'),
                  );
                },
              ),
              BottomMenuItem(
                icon: LucideLightIcons.external_link,
                label: '스페이스에서 열기',
                onTap: () async {
                  unawaited(mixpanel.track('open_document_in_browser', properties: {'via': 'entity_document_menu'}));

                  final url = Uri.parse(entities[index].url);
                  await launchUrl(url, mode: LaunchMode.externalApplication);
                },
              ),
              BottomMenuItem(
                icon: LucideLightIcons.blend,
                label: '공유하기',
                onTap: () async {
                  unawaited(mixpanel.track('open_document_share_modal', properties: {'via': 'entity_document_menu'}));

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
                    GEntityScreen_DuplicateDocument_MutationReq((b) => b..vars.input.documentId = document.id),
                  );

                  unawaited(mixpanel.track('duplicate_document', properties: {'via': 'entity_document_menu'}));
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
                          GEntityScreen_DeleteDocument_MutationReq((b) => b..vars.input.documentId = document.id),
                        );

                        unawaited(mixpanel.track('delete_document', properties: {'via': 'entity_document_menu'}));
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
    }

    Widget buildEntityListItem(BuildContext context, int index) {
      final isSelected = selectedItems.value.contains(entities[index].id);

      return Tappable(
        onTap: () => handleEntityTap(index),
        onLongPress: () async {
          await handleEntityLongPress(index);
        },
        child: Tappable.scale(
          child: buildListRow(
            index: index,
            backgroundColor: isSelecting.value && isSelected
                ? context.colors.accentInfo.withValues(alpha: 0.1)
                : context.colors.surfaceDefault,
            enableDragHandle: true,
            showDivider: index < entities.length - 1,
          ),
        ),
      );
    }

    DragBoundaryDelegate<Rect>? buildReorderDragBoundary(BuildContext listContext) {
      final renderBox = reorderBoundaryKey.currentContext?.findRenderObject() as RenderBox?;
      if (renderBox == null || !renderBox.attached || !renderBox.hasSize) {
        return DragBoundary.forRectMaybeOf(listContext);
      }

      final topLeft = renderBox.localToGlobal(Offset.zero);
      return _RectDragBoundaryDelegate(topLeft & renderBox.size);
    }

    Future<void> handleReorder(int oldIndex, int newIndex) async {
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
    }

    final controlBackgroundColor = context.theme.brightness == Brightness.dark
        ? context.colors.surfaceSubtle
        : context.colors.surfaceDefault;
    final controlShadow = _headingControlShadow(context);
    final isRoot = entity == null;
    final canPop = entity != null && context.router.canPop();
    final shellNavBottomInset = shellNavVisible
        ? MediaQuery.viewPaddingOf(context).bottom + _entityShellNavHeight + _entityShellNavBottomGap
        : 0.0;
    final selectionBarBottomOffset = shellNavVisible
        ? MediaQuery.viewPaddingOf(context).bottom + _entityShellNavHeight + _entityShellNavBottomGap + 12
        : _entitySelectionBarBottomOffset;
    final selectionListBottomPadding =
        selectionBarBottomOffset + _entitySelectionBarHeight + _entitySelectionBarContentGap;
    final reorderViewportTopInset = topInset + OverlayHeading.height;
    final reorderTitleTopPadding = isReordering.value
        ? OverlayHeading.titleTopPadding(context) - reorderViewportTopInset
        : OverlayHeading.titleTopPadding(context);
    final reorderListTopPadding = isReordering.value
        ? _entityHeadingFadeHeight - OverlayHeading.height
        : topInset + _entityHeadingFadeHeight;
    final rootEmptyStateBottomInset = (reorderTitleTopPadding + 40) / 2 + shellNavBottomInset;
    final capsuleTitle = entity == null ? (currentSiteName.value ?? '내 스페이스') : folder!.name;
    final capsuleSubtitle = entity == null ? _buildSpaceSummary(entities) : _buildFolderSummary(folder!);
    final capsuleIcon = entity == null ? LucideLightIcons.folder_open : LucideLightIcons.folder;

    final centerMenuEntries = <_EntityHeadingPopoverEntry>[
      if (entity == null)
        _EntityHeadingPopoverEntry(icon: LucideLightIcons.settings, label: '스페이스 설정', onSelected: openSiteSettings),
      if (entity != null)
        _EntityHeadingPopoverEntry(
          icon: LucideLightIcons.folder_symlink,
          label: '다른 폴더로 옮기기',
          onSelected: moveCurrentEntity,
        ),
      if (entity != null)
        _EntityHeadingPopoverEntry(
          icon: LucideLightIcons.external_link,
          label: '스페이스에서 열기',
          onSelected: openCurrentEntityInSpace,
        ),
      if (entity != null)
        _EntityHeadingPopoverEntry(icon: LucideLightIcons.blend, label: '공유하기', onSelected: openCurrentEntityShare),
      if (entity != null)
        _EntityHeadingPopoverEntry(icon: LucideLightIcons.pen_line, label: '이름 바꾸기', onSelected: renameCurrentEntity),
      if (entity != null)
        _EntityHeadingPopoverEntry(
          icon: LucideLightIcons.trash_2,
          label: '삭제하기',
          onSelected: deleteCurrentEntity,
          iconColor: context.colors.textDanger,
          labelColor: context.colors.textDanger,
        ),
      if (entity == null)
        _EntityHeadingPopoverEntry(
          icon: LucideLightIcons.trash_2,
          label: '휴지통',
          onSelected: () async {
            await context.router.push(TrashRoute());
          },
        ),
    ];

    final editMenuEntries = <_EntityHeadingPopoverEntry>[
      _EntityHeadingPopoverEntry(
        icon: LucideLightIcons.square_check,
        label: '여러 항목 선택하기',
        onSelected: () async {
          startSelectionMode();
        },
      ),
      _EntityHeadingPopoverEntry(
        icon: LucideLightIcons.chevrons_up_down,
        label: '순서 변경하기',
        onSelected: () async {
          isReordering.value = true;
        },
      ),
    ];

    Widget buildCapsuleLabel() {
      return HeadingCapsuleLabel(
        icon: capsuleIcon,
        title: capsuleTitle,
        subtitle: capsuleSubtitle,
        backgroundColor: controlBackgroundColor,
        boxShadow: controlShadow,
        borderRadius: Popover.expandedRadius,
      );
    }

    Widget buildCenterControl() {
      if (isRoot) {
        return OverlayHeadingRevealTitle(
          scrollController: primaryScrollController,
          title: currentSiteName.value ?? '내 스페이스',
        );
      }

      if (isReordering.value || isSelecting.value) {
        return ResponsiveOverlayHeadingCenter(child: buildCapsuleLabel());
      }

      return ResponsiveOverlayHeadingCenter(
        child: Popover(
          position: PopoverPosition.bottomCenter,
          screenPadding: _headingPopoverScreenPadding,
          collapsedBorderRadius: Popover.defaultExpandedBorderRadius,
          backgroundColor: controlBackgroundColor,
          borderSide: BorderSide(color: context.colors.borderStrong),
          anchor: buildCapsuleLabel(),
          pane: _EntityHeadingPopoverPane(
            header: _EntityHeadingPaneHeader(entity: entity, siteName: currentSiteName.value),
            expandToMaxWidth: true,
            entries: centerMenuEntries,
          ),
        ),
      );
    }

    Widget buildRootLeadingControl() {
      return SpacePopoverButton(
        siteName: currentSiteName.value,
        siteLogoUrl: currentSiteLogoUrl.value,
        backgroundColor: controlBackgroundColor,
        boxShadow: controlShadow,
        enabled: !isSelecting.value && !isReordering.value,
        via: 'entity_menu',
      );
    }

    Widget? buildLeadingControl() {
      if (isRoot) {
        return buildRootLeadingControl();
      }

      if (!canPop) {
        return null;
      }

      return HeadingCircleButton(
        icon: LucideLightIcons.chevron_left,
        backgroundColor: controlBackgroundColor,
        boxShadow: controlShadow,
        useSlotHeight: false,
        onTap: () async {
          await context.router.maybePop();
        },
      );
    }

    Widget? buildTrailingControl() {
      if (isSelecting.value) {
        return HeadingCircleButton(
          icon: LucideLightIcons.x,
          backgroundColor: controlBackgroundColor,
          boxShadow: controlShadow,
          useSlotHeight: false,
          onTap: exitSelectionMode,
        );
      }

      if (isReordering.value) {
        return HeadingCircleButton(
          icon: LucideLightIcons.check,
          backgroundColor: controlBackgroundColor,
          boxShadow: controlShadow,
          useSlotHeight: false,
          onTap: () {
            isReordering.value = false;
          },
        );
      }

      if (entities.isEmpty) {
        return null;
      }

      return Popover(
        screenPadding: _headingPopoverScreenPadding,
        collapsedBorderRadius: BorderRadius.circular(Popover.expandedRadius),
        backgroundColor: controlBackgroundColor,
        borderSide: BorderSide(color: context.colors.borderStrong),
        anchor: HeadingCircleButton(
          icon: LucideLightIcons.layout_list,
          backgroundColor: controlBackgroundColor,
          boxShadow: controlShadow,
        ),
        pane: _EntityHeadingPopoverPane(entries: editMenuEntries),
      );
    }

    Widget buildBodyContent() {
      if (isRoot) {
        return CustomScrollView(
          controller: primaryScrollController,
          physics: const AlwaysScrollableScrollPhysics(),
          slivers: [
            SliverToBoxAdapter(
              child: Padding(
                padding: Pad(horizontal: 20, top: reorderTitleTopPadding, bottom: 8),
                child: Text(
                  currentSiteName.value ?? '내 스페이스',
                  style: const TextStyle(fontSize: 24, fontWeight: FontWeight.w800),
                ),
              ),
            ),
            if (entities.isEmpty)
              SliverFillRemaining(
                hasScrollBody: false,
                child: Padding(
                  padding: EdgeInsets.only(bottom: rootEmptyStateBottomInset),
                  child: Center(
                    child: Text(
                      '스페이스가 비어있어요',
                      style: TextStyle(fontSize: 16, fontWeight: FontWeight.w500, color: context.colors.textFaint),
                    ),
                  ),
                ),
              )
            else
              SliverPadding(
                padding: EdgeInsets.fromLTRB(
                  20,
                  4,
                  20,
                  isSelecting.value ? selectionListBottomPadding : _entityListBottomPadding,
                ),
                sliver: SliverHapticReorderableList(
                  orderedIds: [for (final item in entities) item.id],
                  itemBuilder: buildEntityListItem,
                  proxyDecorator: (child, index, animation) => child,
                  dragBoundaryProvider: buildReorderDragBoundary,
                  onReorder: handleReorder,
                ),
              ),
          ],
        );
      }

      if (entities.isEmpty) {
        return Center(
          child: Text(
            '폴더가 비어있어요',
            style: TextStyle(fontSize: 16, fontWeight: FontWeight.w500, color: context.colors.textFaint),
          ),
        );
      }

      return HapticReorderableList(
        orderedIds: [for (final item in entities) item.id],
        controller: primaryScrollController,
        physics: const AlwaysScrollableScrollPhysics(),
        padding: Pad(
          horizontal: 20,
          top: reorderListTopPadding,
          bottom: isSelecting.value ? selectionListBottomPadding : _entityListBottomPadding,
        ),
        itemBuilder: buildEntityListItem,
        proxyDecorator: (child, index, animation) => child,
        dragBoundaryProvider: buildReorderDragBoundary,
        onReorder: handleReorder,
      );
    }

    final bodyContent = isReordering.value
        ? Padding(
            key: reorderBoundaryKey,
            padding: EdgeInsets.only(top: reorderViewportTopInset, bottom: shellNavBottomInset),
            child: buildBodyContent(),
          )
        : SizedBox.expand(key: reorderBoundaryKey, child: buildBodyContent());

    return Screen(
      backgroundColor: context.colors.surfaceSubtle,
      heading: OverlayHeadingBar(
        leading: buildLeadingControl(),
        center: buildCenterControl(),
        trailing: buildTrailingControl(),
      ),
      child: Stack(
        children: [
          Positioned.fill(child: bodyContent),
          if (isReordering.value) ...[
            Positioned(
              top: reorderViewportTopInset,
              left: 0,
              right: 0,
              child: _EntityReorderEdgeFade(
                begin: Alignment.topCenter,
                end: Alignment.bottomCenter,
                baseColor: context.colors.surfaceSubtle,
              ),
            ),
            Positioned(
              left: 0,
              right: 0,
              bottom: shellNavBottomInset,
              child: _EntityReorderEdgeFade(
                begin: Alignment.bottomCenter,
                end: Alignment.topCenter,
                baseColor: context.colors.surfaceSubtle,
              ),
            ),
          ],
          if (isSelecting.value)
            SelectedEntitiesBar(
              bottomOffset: selectionBarBottomOffset,
              isVisible: selectedItems.value.isNotEmpty,
              selectedItems: selectedItems.value,
              entities: entities,
              onClearSelection: () {
                selectedItems.value = {};
              },
              onExitSelectionMode: exitSelectionMode,
            ),
        ],
      ),
    );
  }
}

class _Folder extends StatelessWidget {
  const _Folder(this.entity);

  final GEntityScreen_Entity_entity entity;
  GEntityScreen_Entity_entity_node__asFolder get folder => entity.node as GEntityScreen_Entity_entity_node__asFolder;

  @override
  Widget build(BuildContext context) {
    final parts = <String>[
      if (folder.folderCount > 0) '폴더 ${folder.folderCount.comma}개',
      '문서 ${folder.documentCount.comma}개',
    ];

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(
          children: [
            Icon(LucideLightIcons.folder, size: 16, color: context.colors.accentBrand),
            const Gap(12),
            Expanded(
              child: Text(
                folder.name,
                style: const TextStyle(fontSize: 16, fontWeight: FontWeight.w600),
                overflow: TextOverflow.ellipsis,
                maxLines: 1,
              ),
            ),
          ],
        ),
        Padding(
          padding: const Pad(left: 28),
          child: Text(
            parts.join(' · '),
            style: TextStyle(fontSize: 14, color: context.colors.textFaint),
            overflow: TextOverflow.ellipsis,
            maxLines: 1,
          ),
        ),
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
      children: [
        Row(
          children: [
            Icon(
              document.documentType == GDocumentType.TEMPLATE
                  ? LucideLightIcons.layout_template
                  : LucideLightIcons.file,
              size: 16,
              color: context.colors.textFaint,
            ),
            const Gap(12),
            Expanded(
              child: Text(
                document.title,
                style: const TextStyle(fontSize: 16, fontWeight: FontWeight.w600),
                overflow: TextOverflow.ellipsis,
                maxLines: 1,
              ),
            ),
            const Gap(8),
            Text(
              document.updatedAt.ago,
              style: TextStyle(fontSize: 13, fontWeight: FontWeight.w500, color: context.colors.textDisabled),
            ),
          ],
        ),
        if (document.excerpt.isNotEmpty)
          Padding(
            padding: const Pad(left: 28),
            child: Text(
              document.excerpt,
              style: TextStyle(fontSize: 14, color: context.colors.textFaint),
              overflow: TextOverflow.ellipsis,
              maxLines: 1,
            ),
          ),
      ],
    );
  }
}

class _BottomMenuHeader extends StatelessWidget {
  const _BottomMenuHeader({this.entity, this.siteName});

  final GEntityScreen_Entity_entity? entity;
  final String? siteName;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      spacing: 4,
      children: [
        Row(
          spacing: 16,
          children: [
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
            child: _EntityHeaderMetadata(entity: entity!, siteName: siteName),
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

class _EntityRenameBottomSheet extends HookWidget {
  const _EntityRenameBottomSheet({
    required this.client,
    required this.mixpanel,
    required this.folderId,
    required this.initialName,
  });

  final GraphQLClient client;
  final Mixpanel mixpanel;
  final String folderId;
  final String initialName;

  @override
  Widget build(BuildContext context) {
    final controller = useTextEditingController(text: initialName);
    final inputValue = useValueListenable(controller);
    final nextName = inputValue.text.trim();
    final canSubmit = nextName.isNotEmpty && nextName != initialName;

    useEffect(() {
      WidgetsBinding.instance.addPostFrameCallback((_) {
        controller.selection = TextSelection(baseOffset: 0, extentOffset: controller.text.length);
      });

      return null;
    }, [controller]);

    Future<void> submit() async {
      final name = controller.text.trim();

      if (name.isEmpty) {
        context.toast(ToastType.error, '이름을 입력해주세요.');
        return;
      }

      if (name == initialName) {
        await context.router.maybePop();
        return;
      }

      try {
        await client.request(
          GEntityScreen_RenameFolder_MutationReq(
            (b) => b
              ..vars.input.folderId = folderId
              ..vars.input.name = name,
          ),
        );

        unawaited(mixpanel.track('rename_folder'));

        if (context.mounted) {
          context.toast(ToastType.success, '폴더 이름이 변경되었어요.');
          await context.router.maybePop();
        }
      } catch (_) {
        if (context.mounted) {
          context.toast(ToastType.error, '오류가 발생했어요. 잠시 후 다시 시도해주세요.');
        }
      }
    }

    return ConfirmBottomSheet(
      title: '이름 바꾸기',
      confirmText: '변경',
      shouldDismissOnConfirm: false,
      confirmTextColor: canSubmit ? context.colors.textInverse : context.colors.textFaint,
      confirmBackgroundColor: canSubmit ? context.colors.surfaceInverse : context.colors.surfaceMuted,
      onConfirm: submit,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        spacing: 6,
        children: [
          Text(
            '폴더 이름',
            style: TextStyle(fontSize: 13, fontWeight: FontWeight.w500, color: context.colors.textDefault),
          ),
          TextField(
            controller: controller,
            autofocus: true,
            smartDashesType: SmartDashesType.disabled,
            smartQuotesType: SmartQuotesType.disabled,
            textInputAction: TextInputAction.done,
            decoration: InputDecoration(
              hintText: '폴더 이름',
              hintStyle: TextStyle(color: context.colors.textDisabled),
              border: OutlineInputBorder(borderRadius: BorderRadius.circular(8)),
              contentPadding: const Pad(horizontal: 12, vertical: 10),
            ),
            style: const TextStyle(fontSize: 16),
            onSubmitted: (_) async {
              await submit();
            },
          ),
        ],
      ),
    );
  }
}

class _EntityHeadingPaneHeader extends StatelessWidget {
  const _EntityHeadingPaneHeader({required this.entity, this.siteName});

  final GEntityScreen_WithEntityId_QueryData_entity? entity;
  final String? siteName;

  @override
  Widget build(BuildContext context) {
    if (entity == null) {
      return const SizedBox.shrink();
    }

    final transition = PopoverPaneTransitionScope.maybeOf(context);
    final progress = (transition?.progress ?? 1).clamp(0.0, 1.0);
    final anchorContentRect =
        transition?.anchorContentRect ?? const Rect.fromLTWH(0, 0, 280, HeadingCircleButton.controlSize);
    final sourceIconLeft = anchorContentRect.left + _entityPaneHeaderSourceHorizontalInset;
    final sourceIconTop = anchorContentRect.top + (anchorContentRect.height - _entityPaneHeaderSourceIconSize) / 2;
    const targetIconTop = (_entityPaneHeaderTopHeight - _entityPaneHeaderIconTargetSize) / 2;
    final iconLeft = ui.lerpDouble(sourceIconLeft, _entityPaneHeaderIconTargetLeft, progress);
    final iconTop = ui.lerpDouble(sourceIconTop, targetIconTop, progress);
    final iconSize = ui.lerpDouble(_entityPaneHeaderSourceIconSize, _entityPaneHeaderIconTargetSize, progress);
    final sourceTextLeft =
        anchorContentRect.left +
        _entityPaneHeaderSourceHorizontalInset +
        _entityPaneHeaderSourceIconSize +
        _entityPaneHeaderSourceIconGap;
    final titleFontSize = ui.lerpDouble(14, 17, progress);

    final icon = entity!.node.when(
      folder: (_) => LucideLightIcons.folder,
      document: (doc) =>
          doc.documentType == GDocumentType.TEMPLATE ? LucideLightIcons.layout_template : LucideLightIcons.file,
      orElse: () => throw UnimplementedError(),
    );
    const targetTextLeft =
        _entityPaneHeaderIconTargetLeft + _entityPaneHeaderIconTargetSize + _entityPaneHeaderTitleGap;

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      spacing: 4,
      children: [
        SizedBox(
          height: _entityPaneHeaderTopHeight,
          child: LayoutBuilder(
            builder: (context, constraints) {
              final paneWidth = constraints.maxWidth;
              final sourceTextWidth = math.max<double>(
                0,
                anchorContentRect.width -
                    (_entityPaneHeaderSourceHorizontalInset +
                        _entityPaneHeaderSourceIconSize +
                        _entityPaneHeaderSourceIconGap) -
                    _entityPaneHeaderSourceHorizontalInset,
              );
              final targetTextWidth = math.max<double>(0, paneWidth - targetTextLeft - _entityPaneHeaderTextRightInset);
              final textLeft = ui.lerpDouble(sourceTextLeft, targetTextLeft, progress);
              final textWidth = ui.lerpDouble(sourceTextWidth, targetTextWidth, progress);

              return Stack(
                children: [
                  Positioned(
                    left: 0,
                    top: 0,
                    width: targetTextLeft,
                    height: _entityPaneHeaderTopHeight,
                    child: GestureDetector(
                      behavior: HitTestBehavior.opaque,
                      onTap: () {
                        Popover.close(context);
                      },
                      child: const SizedBox.expand(),
                    ),
                  ),
                  Positioned(
                    left: iconLeft,
                    top: iconTop,
                    width: iconSize,
                    height: iconSize,
                    child: IgnorePointer(
                      child: Stack(
                        alignment: Alignment.centerLeft,
                        children: [
                          Opacity(
                            opacity: 1 - progress,
                            child: Icon(icon, size: iconSize, color: context.colors.textSubtle),
                          ),
                          Opacity(
                            opacity: progress,
                            child: Icon(LucideLightIcons.x, size: iconSize, color: context.colors.textSubtle),
                          ),
                        ],
                      ),
                    ),
                  ),
                  Positioned(
                    left: textLeft,
                    top: 0,
                    width: textWidth,
                    height: _entityPaneHeaderTopHeight,
                    child: Align(
                      alignment: Alignment.centerLeft,
                      child: Text(
                        entity!.node.when(
                          folder: (folder) => folder.name,
                          document: (document) => document.title,
                          orElse: () => throw UnimplementedError(),
                        ),
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: TextStyle(fontSize: titleFontSize, fontWeight: FontWeight.w600, height: 1),
                      ),
                    ),
                  ),
                ],
              );
            },
          ),
        ),
        Padding(
          padding: const EdgeInsets.only(left: targetTextLeft),
          child: _EntityHeaderMetadata(entity: entity!, siteName: siteName),
        ),
      ],
    );
  }
}

class _EntityHeaderMetadata extends StatelessWidget {
  const _EntityHeaderMetadata({required this.entity, this.siteName});

  final GEntityScreen_Entity_entity entity;
  final String? siteName;

  @override
  Widget build(BuildContext context) {
    return Column(
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
                _BottomMenuBreadcrumbLabel(text: siteName ?? '내 스페이스', style: textStyle, maxWidth: maxItemWidth),
                ...entity.ancestors.map(
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
        if (entity.node.when(folder: (folder) => true, document: (document) => true, orElse: () => false))
          Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text(
                entity.visibility == GEntityVisibility.PUBLIC
                    ? '공개'
                    : entity.visibility == GEntityVisibility.UNLISTED &&
                          entity.availability == GEntityAvailability.UNLISTED
                    ? '링크 조회/편집 가능'
                    : entity.visibility == GEntityVisibility.UNLISTED
                    ? '링크 조회 가능'
                    : entity.availability == GEntityAvailability.UNLISTED
                    ? '링크 편집 가능'
                    : '비공개',
                style: TextStyle(
                  fontSize: 14,
                  color:
                      entity.visibility == GEntityVisibility.PUBLIC ||
                          entity.visibility == GEntityVisibility.UNLISTED ||
                          entity.availability == GEntityAvailability.UNLISTED
                      ? context.colors.accentBrand
                      : context.colors.textFaint,
                ),
              ),
              Text(
                entity.node.maybeWhen(
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
    );
  }
}

class _EntityHeadingPopoverEntry {
  const _EntityHeadingPopoverEntry({
    required this.icon,
    required this.label,
    required this.onSelected,
    this.iconColor,
    this.labelColor,
  });

  final IconData icon;
  final String label;
  final Future<void> Function() onSelected;
  final Color? iconColor;
  final Color? labelColor;
}

class _EntityHeadingPopoverPane extends StatelessWidget {
  const _EntityHeadingPopoverPane({required this.entries, this.header, this.expandToMaxWidth = false});

  final List<_EntityHeadingPopoverEntry> entries;
  final Widget? header;
  final bool expandToMaxWidth;

  @override
  Widget build(BuildContext context) {
    final content = Padding(
      padding: EdgeInsets.fromLTRB(
        Popover.panePadding,
        header == null ? Popover.panePadding : Popover.panePadding + 4,
        Popover.panePadding,
        Popover.panePadding,
      ),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          if (header != null) ...[header!, const SizedBox(height: 6)],
          PopoverList(
            indicatorColor: context.colors.surfaceMuted,
            items: [
              for (final entry in entries)
                PopoverListItem(
                  onSelected: () {
                    Popover.close(context);
                    unawaited(entry.onSelected());
                  },
                  child: _EntityHeadingPopoverItem(
                    icon: entry.icon,
                    label: entry.label,
                    iconColor: entry.iconColor,
                    labelColor: entry.labelColor,
                  ),
                ),
            ],
          ),
        ],
      ),
    );

    if (expandToMaxWidth) {
      return ConstrainedBox(constraints: const BoxConstraints(minWidth: 280, maxWidth: 600), child: content);
    }

    return IntrinsicWidth(child: content);
  }
}

class _EntityHeadingPopoverItem extends StatelessWidget {
  const _EntityHeadingPopoverItem({required this.icon, required this.label, this.iconColor, this.labelColor});

  final IconData icon;
  final String label;
  final Color? iconColor;
  final Color? labelColor;

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      height: 42,
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 16),
        child: Row(
          spacing: 12,
          children: [
            Icon(icon, size: 18, color: iconColor ?? context.colors.textDefault),
            Expanded(
              child: Text(
                label,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: TextStyle(
                  fontSize: 15,
                  fontWeight: FontWeight.w500,
                  color: labelColor ?? context.colors.textDefault,
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _EntityCreatePopoverPane extends StatelessWidget {
  const _EntityCreatePopoverPane({required this.onCreateDocument, this.onCreateFolder});

  final Future<void> Function() onCreateDocument;
  final Future<void> Function()? onCreateFolder;

  @override
  Widget build(BuildContext context) {
    return IntrinsicWidth(
      child: Padding(
        padding: const EdgeInsets.all(Popover.panePadding),
        child: PopoverList(
          indicatorColor: context.colors.surfaceMuted,
          items: [
            if (onCreateFolder != null)
              PopoverListItem(
                onSelected: () {
                  ShellTrailingActionMenuScope.dismiss(context);
                  unawaited(onCreateFolder!());
                },
                child: const _EntityCreatePopoverItem(icon: LucideLightIcons.folder_plus, label: '여기에 폴더 만들기'),
              ),
            PopoverListItem(
              onSelected: () {
                ShellTrailingActionMenuScope.dismiss(context);
                unawaited(onCreateDocument());
              },
              child: const _EntityCreatePopoverItem(icon: LucideLightIcons.square_pen, label: '여기에 문서 만들기'),
            ),
          ],
        ),
      ),
    );
  }
}

class _EntityCreatePopoverItem extends StatelessWidget {
  const _EntityCreatePopoverItem({required this.icon, required this.label});

  final IconData icon;
  final String label;

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      height: 42,
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 16),
        child: Row(
          spacing: 12,
          children: [
            Icon(icon, size: 18, color: context.colors.textDefault),
            Expanded(
              child: Text(
                label,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: TextStyle(fontSize: 15, fontWeight: FontWeight.w500, color: context.colors.textDefault),
              ),
            ),
          ],
        ),
      ),
    );
  }
}
