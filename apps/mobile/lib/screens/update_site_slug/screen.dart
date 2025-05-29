import 'package:assorted_layout_widgets/assorted_layout_widgets.dart';
import 'package:auto_route/auto_route.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:luthor/luthor.dart';
import 'package:typie/context/toast.dart';
import 'package:typie/graphql/error.dart';
import 'package:typie/graphql/widget.dart';
import 'package:typie/screens/update_site_slug/__generated__/screen.req.gql.dart';
import 'package:typie/styles/colors.dart';
import 'package:typie/widgets/forms/form.dart';
import 'package:typie/widgets/forms/text_field.dart';
import 'package:typie/widgets/heading.dart';
import 'package:typie/widgets/screen.dart';
import 'package:typie/widgets/tappable.dart';

@RoutePage()
class UpdateSiteSlugScreen extends HookWidget {
  const UpdateSiteSlugScreen({super.key});

  @override
  Widget build(BuildContext context) {
    return Screen(
      heading: const Heading(title: '사이트 주소 변경'),
      resizeToAvoidBottomInset: true,
      padding: const Pad(top: 20),
      child: GraphQLOperation(
        operation: GUpdateSiteSlugScreen_QueryReq(),
        builder: (context, client, data) {
          final unavailableSiteSlugs = ['admin', 'app', 'cname', 'dev', 'docs', 'help', 'template', 'www'];

          return HookForm(
            schema: l.schema({
              'slug': l
                  .string()
                  .min(4, message: '사이트 주소는 4글자 이상이여야 해요')
                  .max(63, message: '사이트 주소는 63글자를 넘을 수 없어요')
                  .regex(r'^[\da-z-]+$', message: '사이트 주소는 소문자, 숫자, 하이픈만 사용할 수 있어요')
                  .regex(r'^(?!.*--)[\da-z-]+$', message: '하이픈을 연속으로 사용할 수 없어요')
                  .regex(r'^[\da-z][\da-z-]*[\da-z]$', message: '사이트 주소는 하이픈으로 시작하거나 끝날 수 없어요')
                  .custom((value) => !unavailableSiteSlugs.contains(value), message: '사용할 수 없는 사이트 주소에요')
                  .required(message: '사이트 주소를 입력해 주세요'),
            }),
            onSubmit: (form) async {
              try {
                await client.request(
                  GUpdateSiteSlugScreen_UpdateSiteSlug_MutationReq(
                    (b) => b
                      ..vars.input.slug = form.data['slug'] as String
                      ..vars.input.siteId = data.me!.sites[0].id,
                  ),
                );

                if (context.mounted) {
                  context.toast(ToastType.success, '사이트 주소가 변경되었어요.');
                  await context.router.maybePop();
                }
              } on TypieError catch (e) {
                if (context.mounted) {
                  switch (e.code) {
                    case 'site_slug_already_exists':
                      form.setError('slug', '이미 존재하는 사이트 주소예요.');
                    default:
                      context.toast(ToastType.error, '오류가 발생했어요. 잠시 후 다시 시도해주세요.', bottom: 64);
                  }
                }
              }
            },
            builder: (context, form) {
              return Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                spacing: 16,
                children: [
                  Padding(
                    padding: const Pad(horizontal: 20),
                    child: HookFormTextField(
                      name: 'slug',
                      label: '사이트 주소',
                      placeholder: '사이트 주소',
                      initialValue: data.me!.sites[0].slug,
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
