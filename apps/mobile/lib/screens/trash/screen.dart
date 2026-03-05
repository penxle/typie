import 'dart:async';

import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:mixpanel_flutter/mixpanel_flutter.dart';
import 'package:typie/context/bottom_sheet.dart';
import 'package:typie/context/modal.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/context/toast.dart';
import 'package:typie/extensions/jiffy.dart';
import 'package:typie/graphql/client.dart';
import 'package:typie/graphql/widget.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/icons/typie.dart';
import 'package:typie/routers/app.gr.dart';
import 'package:typie/screens/trash/__generated__/entity_fragment.data.gql.dart';
import 'package:typie/screens/trash/__generated__/purge_entities_mutation.req.gql.dart';
import 'package:typie/screens/trash/__generated__/recover_entity_mutation.data.gql.dart';
import 'package:typie/screens/trash/__generated__/recover_entity_mutation.req.gql.dart';
import 'package:typie/screens/trash/__generated__/screen_with_entity_id_query.data.gql.dart';
import 'package:typie/screens/trash/__generated__/screen_with_entity_id_query.req.gql.dart';
import 'package:typie/screens/trash/__generated__/screen_with_site_id_query.req.gql.dart';
import 'package:typie/services/preference.dart';
import 'package:typie/widgets/heading.dart';
import 'package:typie/widgets/screen.dart';

@RoutePage()
class TrashScreen extends StatelessWidget {
  const TrashScreen({super.key, @PathParam() this.entityId});

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
    final pref = useService<Pref>();

    return GraphQLOperation(
      initialBackgroundColor: context.colors.surfaceSubtle,
      operation: GTrashScreen_WithSiteId_QueryReq((b) => b..vars.siteId = pref.siteId),
      builder: (context, client, data) {
        return _TrashList(null, data.site.deletedEntities.toList(), data.site.name);
      },
    );
  }
}

class _WithEntityId extends StatelessWidget {
  const _WithEntityId(this.entityId);

  final String entityId;

  @override
  Widget build(BuildContext context) {
    return GraphQLOperation(
      initialBackgroundColor: context.colors.surfaceSubtle,
      operation: GTrashScreen_WithEntityId_QueryReq((b) => b..vars.entityId = entityId),
      builder: (context, client, data) {
        return _TrashList(data.entity, data.entity.deletedChildren.toList(), data.entity.site.name);
      },
    );
  }
}

class _TrashList extends HookWidget {
  const _TrashList(this.entity, this.entities, this.siteName);

  final GTrashScreen_WithEntityId_QueryData_entity? entity;
  final List<GTrashScreen_Entity_entity> entities;
  final String siteName;

  GTrashScreen_WithEntityId_QueryData_entity_node__asFolder? get folder =>
      entity?.node as GTrashScreen_WithEntityId_QueryData_entity_node__asFolder?;

  @override
  Widget build(BuildContext context) {
    final client = useService<GraphQLClient>();
    final mixpanel = useService<Mixpanel>();
    final primaryScrollController = PrimaryScrollController.of(context);

    String getEntityTitle(GTrashScreen_Entity_entity entity) {
      return entity.node.when(
        folder: (folder) => folder.name,
        document: (document) => document.title,
        orElse: () => '',
      );
    }

    String getEntityType(GTrashScreen_Entity_entity entity) {
      return entity.node.when(folder: (_) => '폴더', document: (_) => '문서', orElse: () => '');
    }

    String getEntityTypename(GTrashScreen_Entity_entity entity) {
      return entity.node.G__typename.toLowerCase();
    }

    String getRecoveredPath(GTrashScreen_RecoverEntity_MutationData_recoverEntity entity) {
      final path = <String>[
        ...entity.ancestors.map((ancestor) => ancestor.node.when(folder: (folder) => folder.name, orElse: () => '')),
        entity.node.when(folder: (folder) => folder.name, document: (document) => document.title, orElse: () => ''),
      ].where((segment) => segment.isNotEmpty).join(' › ');

      return path;
    }

    Future<void> recoverEntity(
      GTrashScreen_Entity_entity entity, {
      String via = 'trash',
      bool shouldPop = false,
    }) async {
      final type = getEntityType(entity);
      final typename = getEntityTypename(entity);
      try {
        final resp = await client.request(
          GTrashScreen_RecoverEntity_MutationReq((b) => b..vars.input.entityId = entity.id),
        );
        final restoredPath = getRecoveredPath(resp.recoverEntity);

        unawaited(mixpanel.track('recover_entity', properties: {'via': via, 'type': typename.toLowerCase()}));

        if (context.mounted) {
          context.toast(ToastType.success, '"$restoredPath" $type를 복원했어요');
          if (shouldPop) {
            await context.router.maybePop();
          }
        }
      } catch (_) {
        if (context.mounted) {
          context.toast(ToastType.error, '오류가 발생했어요. 잠시 후 다시 시도해주세요.', bottom: 64);
        }
      }
    }

    Future<void> purgeEntity(GTrashScreen_Entity_entity entity, {String via = 'trash', bool shouldPop = false}) async {
      final title = getEntityTitle(entity);
      final type = getEntityType(entity);
      final typename = getEntityTypename(entity);
      await context.showModal(
        child: ConfirmModal(
          title: '$type 영구 삭제',
          message: '영구 삭제한 $type는 복원할 수 없어요. 정말 삭제하시겠어요?',
          confirmText: '삭제',
          confirmTextColor: context.colors.textBright,
          confirmBackgroundColor: context.colors.accentDanger,
          onConfirm: () async {
            try {
              await client.request(
                GTrashScreen_PurgeEntities_MutationReq((b) => b..vars.input.entityIds.add(entity.id)),
              );

              unawaited(mixpanel.track('purge_entity', properties: {'via': via, 'type': typename.toLowerCase()}));

              if (context.mounted) {
                context.toast(ToastType.success, '"$title" $type가 영구 삭제되었어요.');
                if (shouldPop) {
                  await context.router.maybePop();
                }
              }
            } catch (_) {
              if (context.mounted) {
                context.toast(ToastType.error, '오류가 발생했어요. 잠시 후 다시 시도해주세요.', bottom: 64);
              }
            }
          },
        ),
      );
    }

    Future<void> showEntityMenu(GTrashScreen_Entity_entity entity) async {
      await context.showBottomSheet(
        child: BottomMenu(
          header: _BottomMenuHeader(entity: entity, siteName: siteName),
          items: [
            BottomMenuItem(
              label: '복원',
              icon: LucideLightIcons.undo_2,
              onTap: () async {
                await recoverEntity(entity);
              },
            ),
            BottomMenuItem(
              label: '영구 삭제',
              icon: LucideLightIcons.trash_2,
              iconColor: context.colors.textDanger,
              labelColor: context.colors.textDanger,
              onTap: () async {
                await purgeEntity(entity);
              },
            ),
          ],
        ),
      );
    }

    return Screen(
      heading: Heading(
        titleWidget: Row(
          spacing: 8,
          children: [
            const Icon(LucideLightIcons.trash_2, size: 20),
            Expanded(
              child: Text(
                entity == null ? '휴지통' : folder!.name,
                style: const TextStyle(fontSize: 18, fontWeight: FontWeight.w600),
                overflow: TextOverflow.ellipsis,
              ),
            ),
          ],
        ),
        actions: [
          HeadingAction(
            icon: LucideLightIcons.ellipsis,
            onTap: () async {
              await context.showBottomSheet(
                child: BottomMenu(
                  header: _BottomMenuHeader(entity: entity, siteName: siteName),
                  items: [
                    if (entity != null) ...[
                      BottomMenuItem(
                        label: '복원',
                        icon: LucideLightIcons.undo_2,
                        onTap: () async {
                          await recoverEntity(entity!, via: 'trash_menu', shouldPop: true);
                        },
                      ),
                      BottomMenuItem(
                        label: '영구 삭제',
                        icon: LucideLightIcons.trash_2,
                        iconColor: context.colors.textDanger,
                        labelColor: context.colors.textDanger,
                        onTap: () async {
                          await purgeEntity(entity!, via: 'trash_menu', shouldPop: true);
                        },
                      ),
                    ],
                    BottomMenuItem(
                      label: entity == null ? '휴지통 비우기' : '폴더 비우기',
                      icon: LucideLightIcons.brush_cleaning,
                      iconColor: context.colors.textDanger,
                      labelColor: context.colors.textDanger,
                      onTap: () async {
                        if (entities.isEmpty) {
                          if (context.mounted) {
                            if (entity == null) {
                              context.toast(ToastType.notification, '휴지통이 비어있어요');
                            } else {
                              context.toast(ToastType.notification, '폴더가 비어있어요');
                            }
                          }
                          return;
                        }

                        await context.showModal(
                          child: ConfirmModal(
                            title: entity == null ? '휴지통 비우기' : '폴더 비우기',
                            message: entity == null
                                ? '휴지통에 있는 ${entities.length}개 항목을 모두 영구 삭제할까요? 삭제된 항목은 복원할 수 없어요.'
                                : '이 폴더에 있는 ${entities.length}개 항목을 모두 영구 삭제할까요? 삭제된 항목은 복원할 수 없어요.',
                            confirmText: '비우기',
                            confirmTextColor: context.colors.textBright,
                            confirmBackgroundColor: context.colors.accentDanger,
                            onConfirm: () async {
                              try {
                                await client.request(
                                  GTrashScreen_PurgeEntities_MutationReq(
                                    (b) => b..vars.input.entityIds.addAll(entities.map((e) => e.id)),
                                  ),
                                );

                                unawaited(
                                  mixpanel.track(
                                    'purge_entities',
                                    properties: {'via': 'trash', 'count': entities.length},
                                  ),
                                );

                                if (context.mounted) {
                                  context.toast(ToastType.success, '휴지통을 비웠어요');
                                }
                              } catch (_) {
                                if (context.mounted) {
                                  context.toast(ToastType.error, '오류가 발생했어요. 잠시 후 다시 시도해주세요.', bottom: 64);
                                }
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
      ),
      child: entities.isEmpty
          ? Center(
              child: Text(
                entity == null ? '휴지통이 비어있어요' : '폴더가 비어있어요',
                style: TextStyle(fontSize: 16, fontWeight: FontWeight.w500, color: context.colors.textFaint),
              ),
            )
          : ListView(
              controller: primaryScrollController,
              padding: const Pad(horizontal: 20, vertical: 14),
              children: entities.map((entity) {
                return Padding(
                  padding: const Pad(vertical: 6),
                  child: GestureDetector(
                    onTap: () async {
                      await entity.node.when(
                        folder: (folder) async {
                          await context.router.push(TrashRoute(entityId: entity.id));
                        },
                        document: (_) async {
                          await showEntityMenu(entity);
                        },
                        orElse: () => throw UnimplementedError(),
                      );
                    },
                    onLongPress: () async {
                      await entity.node.when(
                        folder: (folder) async {
                          await showEntityMenu(entity);
                        },
                        document: (_) async {
                          await showEntityMenu(entity);
                        },
                        orElse: () => throw UnimplementedError(),
                      );
                    },
                    child: IntrinsicHeight(
                      child: Container(
                        decoration: BoxDecoration(
                          border: Border.all(color: context.colors.borderStrong),
                          borderRadius: const BorderRadius.all(Radius.circular(8)),
                          color: context.colors.surfaceDefault,
                        ),
                        child: Padding(
                          padding: const Pad(horizontal: 16, vertical: 12),
                          child: entity.node.when(
                            folder: (_) => _Folder(entity),
                            document: (_) => _Document(entity),
                            orElse: () => throw UnimplementedError(),
                          ),
                        ),
                      ),
                    ),
                  ),
                );
              }).toList(),
            ),
    );
  }
}

class _Folder extends StatelessWidget {
  const _Folder(this.entity);

  final GTrashScreen_Entity_entity entity;
  GTrashScreen_Entity_entity_node__asFolder get folder => entity.node as GTrashScreen_Entity_entity_node__asFolder;

  @override
  Widget build(BuildContext context) {
    return Row(
      spacing: 8,
      children: [
        const Icon(TypieIcons.folder_filled, size: 18),
        Expanded(
          child: Text(
            folder.name,
            style: const TextStyle(fontSize: 16, fontWeight: FontWeight.w500),
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

  final GTrashScreen_Entity_entity entity;
  GTrashScreen_Entity_entity_node__asDocument get document =>
      entity.node as GTrashScreen_Entity_entity_node__asDocument;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      spacing: 4,
      children: [
        Row(
          spacing: 8,
          children: [
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
  const _BottomMenuHeader({this.entity, this.siteName});

  final GTrashScreen_Entity_entity? entity;
  final String? siteName;

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Row(
          spacing: 16,
          children: [
            Icon(
              entity?.node.when(
                    folder: (_) => LucideLightIcons.folder,
                    document: (_) => LucideLightIcons.file,
                    orElse: () => throw UnimplementedError(),
                  ) ??
                  LucideLightIcons.trash_2,
              size: 20,
            ),
            Expanded(
              child: Text(
                entity?.node.when(
                      folder: (folder) => folder.name,
                      document: (document) => document.title,
                      orElse: () => throw UnimplementedError(),
                    ) ??
                    '휴지통',
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
                Text('삭제됨', style: TextStyle(fontSize: 14, color: context.colors.textFaint)),
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
