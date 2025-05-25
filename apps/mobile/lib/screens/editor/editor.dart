import 'dart:io';

import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/context/bottom_sheet.dart';
import 'package:typie/context/modal.dart';
import 'package:typie/env.dart';
import 'package:typie/graphql/widget.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/screens/editor/__generated__/editor.req.gql.dart';
import 'package:typie/screens/editor/schema.dart';
import 'package:typie/screens/editor/scope.dart';
import 'package:typie/screens/editor/toolbar.dart';
import 'package:typie/services/auth.dart';
import 'package:typie/services/keyboard.dart';
import 'package:typie/styles/colors.dart';
import 'package:typie/widgets/heading.dart';
import 'package:typie/widgets/screen.dart';
import 'package:typie/widgets/webview.dart';

class Editor extends HookWidget {
  const Editor({required this.slug, super.key});

  final String slug;

  @override
  Widget build(BuildContext context) {
    final auth = useService<Auth>();
    final keyboard = useService<Keyboard>();

    final isReady = useState(false);

    final scope = EditorStateScope.of(context);
    final webViewController = useValueListenable(scope.webViewController);

    useEffect(() {
      final subscription = keyboard.onHeightChange.listen((height) {
        if (height > 0) {
          scope.keyboardHeight.value = height;
          scope.selectedToolboxIdx.value = -1;
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
        }
      });

      return subscription.cancel;
    }, [webViewController]);

    return GraphQLOperation(
      operation: GEditorScreen_QueryReq((b) => b..vars.slug = slug),
      builder: (context, client, data) {
        return Screen(
          heading: Heading(
            title: data.post.title,
            actions: [
              HeadingAction(
                icon: LucideLightIcons.ellipsis,
                onTap: () async {
                  await context.showBottomSheet(
                    child: BottomMenu(
                      items: [
                        BottomMenuItem(
                          icon: LucideLightIcons.trash,
                          label: '삭제하기',
                          onTap: () async {
                            await context.showModal(
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
          safeArea: false,
          keyboardDismiss: false,
          child: Stack(
            fit: StackFit.expand,
            children: [
              Opacity(
                opacity: isReady.value ? 1 : 0.01,
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: [
                    Expanded(
                      child: WebView(
                        initialUrl: '${Env.websiteUrl}/_webview/editor?slug=$slug',
                        initialCookies: [Cookie('typie-at', (auth.value as Authenticated).accessToken)],
                        onWebViewCreated: (controller) {
                          scope.webViewController.value = controller;
                        },
                      ),
                    ),
                    const EditorToolbar(),
                  ],
                ),
              ),
              if (!isReady.value)
                const Positioned.fill(
                  child: Center(child: CircularProgressIndicator(color: AppColors.gray_950)),
                ),
            ],
          ),
        );
      },
    );
  }
}
