import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/context/modal.dart';
import 'package:typie/graphql/client.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/screens/editor/__generated__/unfurl_embed.req.gql.dart';
import 'package:typie/screens/editor/scope.dart';
import 'package:typie/screens/editor/toolbar/buttons/floating.dart';
import 'package:typie/widgets/forms/form.dart';
import 'package:typie/widgets/forms/text_field.dart';

class EmbedFloatingToolbar extends HookWidget {
  const EmbedFloatingToolbar({super.key});

  @override
  Widget build(BuildContext context) {
    final client = useService<GraphQLClient>();
    final scope = EditorStateScope.of(context);
    final proseMirrorState = useValueListenable(scope.proseMirrorState);

    return Row(
      spacing: 8,
      children: [
        if (proseMirrorState!.currentNode!.attrs?['id'] == null)
          FloatingToolbarButton(
            icon: LucideLightIcons.file_up,
            onTap: () async {
              final nodeId = proseMirrorState.currentNode!.attrs?['nodeId'] as String?;
              if (nodeId == null) {
                return;
              }

              await context.showModal(
                intercept: true,
                child: HookForm(
                  onSubmit: (form) async {
                    final embedUrl = form.data['url'] as String;
                    final url = RegExp(r'^[^:]+:\/\/').hasMatch(embedUrl) ? embedUrl : 'https://$embedUrl';

                    await scope.webViewController.value?.emitEvent('nodeview', {
                      'nodeId': nodeId,
                      'name': 'inflight',
                      'detail': {'url': url},
                    });

                    try {
                      final result = await client.request(
                        GEditorScreen_UnfurlEmbed_MutationReq((b) => b..vars.input.url = url),
                      );

                      await scope.webViewController.value?.emitEvent('nodeview', {
                        'nodeId': nodeId,
                        'name': 'success',
                        'detail': {
                          'attrs': {
                            'id': result.unfurlEmbed.id,
                            'url': result.unfurlEmbed.url,
                            'title': result.unfurlEmbed.title,
                            'description': result.unfurlEmbed.description,
                            'thumbnailUrl': result.unfurlEmbed.thumbnailUrl,
                            'html': result.unfurlEmbed.html,
                          },
                        },
                      });
                    } catch (_) {
                      await scope.webViewController.value?.emitEvent('nodeview', {'nodeId': nodeId, 'name': 'error'});
                    }
                  },
                  builder: (context, form) {
                    return ConfirmModal(
                      title: '임베드 삽입',
                      confirmText: '삽입',
                      onConfirm: () async {
                        await form.submit();
                      },
                      child: const HookFormTextField.collapsed(
                        name: 'url',
                        placeholder: 'https://...',
                        style: TextStyle(fontSize: 16),
                        autofocus: true,
                        submitOnEnter: false,
                        keyboardType: TextInputType.url,
                      ),
                    );
                  },
                ),
              );
            },
          ),
        FloatingToolbarButton(
          icon: LucideLightIcons.trash_2,
          onTap: () async {
            await scope.command('delete');
          },
        ),
      ],
    );
  }
}
