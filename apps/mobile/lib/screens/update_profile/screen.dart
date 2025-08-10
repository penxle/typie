import 'dart:io';

import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:auto_route/auto_route.dart';
import 'package:built_value/json_object.dart';
import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:luthor/luthor.dart';
import 'package:mixpanel_flutter/mixpanel_flutter.dart';
import 'package:transparent_image/transparent_image.dart';
import 'package:typie/context/theme.dart';
import 'package:typie/context/toast.dart';
import 'package:typie/graphql/widget.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/screens/update_profile/__generated__/persist_blob_as_image_mutation.req.gql.dart';
import 'package:typie/screens/update_profile/__generated__/screen_query.req.gql.dart';
import 'package:typie/screens/update_profile/__generated__/update_user_mutation.req.gql.dart';
import 'package:typie/services/blob.dart';
import 'package:typie/widgets/forms/form.dart';
import 'package:typie/widgets/forms/text_field.dart';
import 'package:typie/widgets/heading.dart';
import 'package:typie/widgets/screen.dart';
import 'package:typie/widgets/tappable.dart';

@RoutePage()
class UpdateProfileScreen extends HookWidget {
  const UpdateProfileScreen({super.key});

  @override
  Widget build(BuildContext context) {
    final blob = useService<Blob>();
    final form = useHookForm();
    final mixpanel = useService<Mixpanel>();

    return Screen(
      heading: const Heading(title: '프로필 변경'),
      resizeToAvoidBottomInset: true,
      bottomAction: BottomAction(
        text: '변경',
        onTap: () async {
          await form.submit();
        },
      ),
      child: GraphQLOperation(
        operation: GUpdateProfileScreen_QueryReq(),
        builder: (context, client, data) {
          final avatarId = useState(data.me!.avatar.id);
          final avatarUrl = useState<String>(data.me!.avatar.url);

          return HookForm(
            form: form,
            schema: l.schema({
              'name': l
                  .string()
                  .min(1, message: '닉네임을 입력해주세요')
                  .max(20, message: '닉네임은 20자를 넘을 수 없어요')
                  .required(message: '닉네임을 입력해주세요'),
              'avatarId': l.string(),
            }),
            onSubmit: (form) async {
              try {
                await client.request(
                  GUpdateProfileScreen_UpdateUser_MutationReq(
                    (b) => b
                      ..vars.input.avatarId = avatarId.value
                      ..vars.input.name = form.data['name'] as String,
                  ),
                );

                await mixpanel.track('update_user');

                if (context.mounted) {
                  await context.router.maybePop();
                }
              } catch (_) {
                if (context.mounted) {
                  context.toast(ToastType.error, '프로필 변경에 실패했어요. 다시 시도해주세요.');
                }
              }
            },
            builder: (context, form) {
              return Column(
                spacing: 24,
                children: [
                  Padding(
                    padding: const Pad(top: 48),
                    child: Tappable(
                      onTap: () async {
                        final result = await FilePicker.platform.pickFiles(type: FileType.image);
                        if (result == null) {
                          return;
                        }

                        final pickedFile = result.files.firstOrNull;
                        if (pickedFile == null) {
                          return;
                        }

                        final file = File(pickedFile.path!);

                        final path = await blob.upload(file);
                        final resp = await client.request(
                          GUpdateProfileScreen_PersistBlobAsImage_MutationReq(
                            (b) => b
                              ..vars.input.path = path
                              ..vars.input.modification = JsonObject({
                                'resize': {'width': 512, 'height': 512, 'fit': 'cover', 'withoutEnlargement': true},
                                'format': 'png',
                              }),
                          ),
                        );

                        avatarUrl.value = resp.persistBlobAsImage.url;
                        avatarId.value = resp.persistBlobAsImage.id;
                      },
                      child: Stack(
                        children: [
                          ClipOval(
                            child: FadeInImage.memoryNetwork(
                              placeholder: kTransparentImage,
                              image: avatarUrl.value,
                              width: 80,
                              height: 80,
                              fit: BoxFit.cover,
                              fadeInDuration: const Duration(milliseconds: 150),
                            ),
                          ),
                          Container(
                            width: 80,
                            height: 80,
                            decoration: BoxDecoration(
                              color: context.colors.textDefault.withValues(alpha: 0.15),
                              shape: BoxShape.circle,
                            ),
                            child: Icon(LucideLightIcons.camera, size: 28, color: context.colors.textBright),
                          ),
                        ],
                      ),
                    ),
                  ),
                  Padding(
                    padding: const Pad(horizontal: 20),
                    child: HookFormTextField(
                      name: 'name',
                      label: '닉네임',
                      placeholder: '닉네임',
                      initialValue: data.me!.name,
                      autofocus: true,
                    ),
                  ),
                ],
              );
            },
          );
        },
      ),
    );
  }
}
