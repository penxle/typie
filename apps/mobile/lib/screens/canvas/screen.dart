import 'dart:async';

import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:mixpanel_flutter/mixpanel_flutter.dart';
import 'package:typie/context/bottom_sheet.dart';
import 'package:typie/context/modal.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/context/toast.dart';
import 'package:typie/graphql/widget.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/routers/app.gr.dart';
import 'package:typie/screens/canvas/__generated__/canvas_query.data.gql.dart';
import 'package:typie/screens/canvas/__generated__/canvas_query.req.gql.dart';
import 'package:typie/screens/canvas/__generated__/delete_canvas_mutation.req.gql.dart';
import 'package:typie/screens/canvas/__generated__/duplicate_canvas_mutation.req.gql.dart';
import 'package:typie/screens/canvas/canvas_viewer.dart';
import 'package:typie/screens/canvas/scope.dart';
import 'package:typie/widgets/heading.dart';
import 'package:typie/widgets/screen.dart';
import 'package:typie/widgets/webview.dart';

@RoutePage()
class CanvasScreen extends HookWidget {
  const CanvasScreen({super.key, @PathParam() required this.slug});

  final String slug;

  @override
  Widget build(BuildContext context) {
    final mixpanel = useService<Mixpanel>();

    final webViewController = useValueNotifier<WebViewController?>(null);

    return GraphQLOperation(
      initialBackgroundColor: context.colors.surfaceSubtle,
      operation: GCanvasScreen_QueryReq((b) => b..vars.slug = slug),
      builder: (context, client, data) {
        if (data.entity.node.G__typename != 'Canvas') {
          return Screen(
            heading: const Heading(title: '캔버스'),
            child: Center(
              child: Text('캔버스를 찾을 수 없어요', style: TextStyle(fontSize: 16, color: context.colors.textFaint)),
            ),
          );
        }

        final canvas = data.entity.node as GCanvasScreen_QueryData_entity_node__asCanvas;

        return Screen(
          heading: Heading(
            titleWidget: Row(
              spacing: 8,
              children: [
                const Icon(LucideLightIcons.line_squiggle, size: 20),
                Expanded(
                  child: Text(
                    canvas.title,
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
                      items: [
                        BottomMenuItem(
                          icon: LucideLightIcons.copy,
                          label: '복제하기',
                          onTap: () async {
                            try {
                              final response = await client.request(
                                GCanvasScreen_DuplicateCanvas_MutationReq((b) => b..vars.input.canvasId = canvas.id),
                              );

                              unawaited(mixpanel.track('duplicate_canvas', properties: {'via': 'canvas_menu'}));

                              if (context.mounted) {
                                await context.router.popAndPush(
                                  CanvasRoute(slug: response.duplicateCanvas.entity.slug),
                                );
                              }
                            } catch (_) {
                              if (context.mounted) {
                                context.toast(ToastType.error, '캔버스 복제에 실패했습니다');
                              }
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
                                title: '캔버스 삭제',
                                message: '"${canvas.title}" 캔버스를 삭제하시겠어요?',
                                confirmText: '삭제하기',
                                confirmTextColor: context.colors.textBright,
                                confirmBackgroundColor: context.colors.accentDanger,
                                onConfirm: () async {
                                  try {
                                    await client.request(
                                      GCanvasScreen_DeleteCanvas_MutationReq((b) => b..vars.input.canvasId = canvas.id),
                                    );

                                    unawaited(mixpanel.track('delete_canvas', properties: {'via': 'canvas_menu'}));
                                    if (context.mounted) {
                                      await context.router.maybePop();
                                    }
                                  } catch (_) {
                                    if (context.mounted) {
                                      context.toast(ToastType.error, '캔버스 삭제에 실패했습니다');
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
          child: CanvasViewerStateScope(
            webViewController: webViewController,
            child: CanvasViewer(siteId: data.entity.site.id, slug: slug),
          ),
        );
      },
    );
  }
}
