import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:typie/graphql/client.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/screens/editor/__generated__/unfurl_embed.req.gql.dart';
import 'package:typie/screens/editor/scope.dart';
import 'package:typie/styles/colors.dart';
import 'package:typie/widgets/tappable.dart';

class EmbedToolbar extends HookWidget {
  const EmbedToolbar({super.key});

  @override
  Widget build(BuildContext context) {
    final scope = EditorStateScope.of(context);
    final proseMirrorState = useValueListenable(scope.proseMirrorState);
    final client = useService<GraphQLClient>();

    final embedUrl = useState('');

    return Padding(
      padding: const Pad(horizontal: 20),
      child: Row(
        spacing: 8,
        children: [
          Expanded(
            child: TextField(
              autofocus: true,
              smartDashesType: SmartDashesType.disabled,
              smartQuotesType: SmartQuotesType.disabled,
              decoration: const InputDecoration.collapsed(
                hintText: 'https://...',
                hintStyle: TextStyle(fontSize: 16, fontWeight: FontWeight.w500, color: AppColors.gray_300),
              ),
              onChanged: (value) {
                embedUrl.value = value;
              },
            ),
          ),
          Tappable(
            onTap: () async {
              final nodeId = proseMirrorState?.currentNode!.attrs?['nodeId'] as String?;
              if (nodeId == null) {
                return;
              }

              final url = RegExp(r'^[^:]+:\/\/').hasMatch(embedUrl.value)
                  ? embedUrl.value
                  : 'https://${embedUrl.value}';

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
            child: const Text('확인'),
          ),
        ],
      ),
    );
  }
}
