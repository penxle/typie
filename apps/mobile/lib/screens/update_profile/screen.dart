import 'dart:io';

import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:auto_route/auto_route.dart';
import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:luthor/luthor.dart';
import 'package:transparent_image/transparent_image.dart';
import 'package:typie/context/toast.dart';
import 'package:typie/graphql/widget.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/icons/lucide_light.dart';
import 'package:typie/screens/update_profile/__generated__/screen.req.gql.dart';
import 'package:typie/services/blob.dart';
import 'package:typie/styles/colors.dart';
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

    return Screen(
      heading: const Heading(title: '프로필 변경'),
      resizeToAvoidBottomInset: true,
      child: GraphQLOperation(
        operation: GUpdateProfileScreen_QueryReq(),
        builder: (context, client, data) {
          final avatarId = useState(data.me!.avatar.id);
          final avatarUrl = useState<String>(data.me!.avatar.url);

          return HookForm(
            schema: l.schema({
              'name': l.string().min(1, message: '이름을 입력해주세요.')
                ..max(20, message: '이름은 20자를 넘을 수 없어요').required(message: '이름을 입력해주세요.'),
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
                          GUpdateProfileScreen_PersistBlobAsImage_MutationReq((b) => b..vars.input.path = path),
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
                              color: AppColors.gray_950.withValues(alpha: 0.15),
                              shape: BoxShape.circle,
                            ),
                            child: const Icon(LucideLightIcons.camera, size: 28, color: AppColors.white),
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
                  const Spacer(),
                  Tappable(
                    child: Container(
                      alignment: Alignment.center,
                      decoration: const BoxDecoration(color: AppColors.gray_950),
                      padding: Pad(vertical: 16, bottom: MediaQuery.paddingOf(context).bottom),
                      child: const Text(
                        '변경',
                        style: TextStyle(fontSize: 16, fontWeight: FontWeight.w700, color: AppColors.white),
                      ),
                    ),
                    onTap: () async {
                      await form.submit();
                    },
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
