import 'dart:async';
import 'dart:io';

import 'package:firebase_messaging/firebase_messaging.dart';
import 'package:flutter/material.dart';
import 'package:flutter_hooks/flutter_hooks.dart';
import 'package:permission_handler/permission_handler.dart';
import 'package:typie/context/toast.dart';
import 'package:typie/firebase/__generated__/push_notification_register_push_notification_token_mutation.req.gql.dart';
import 'package:typie/graphql/client.dart';
import 'package:typie/hooks/service.dart';
import 'package:typie/services/auth.dart';

class PushNotification extends HookWidget {
  const PushNotification({required this.child, super.key});

  final Widget child;

  @override
  Widget build(BuildContext context) {
    final auth = useService<Auth>();
    final client = useService<GraphQLClient>();
    final authState = useValueListenable(auth);

    useEffect(() {
      if (authState is Authenticated) {
        unawaited(_registerToken(client));
      } else {
        unawaited(_deleteToken());
      }

      return null;
    }, [authState]);

    useEffect(() {
      final subscription = FirebaseMessaging.onMessage.listen((message) {
        final title = message.notification?.title;
        if (context.mounted && title != null) {
          context.toast(ToastType.notification, title, duration: const Duration(seconds: 10));
        }
      });

      return subscription.cancel;
    });

    return child;
  }
}

Future<void> _registerToken(GraphQLClient client) async {
  try {
    final status = await Permission.notification.request();
    if (!status.isGranted) {
      return;
    }

    if (Platform.isIOS) {
      final token = await FirebaseMessaging.instance.getAPNSToken();
      if (token == null) {
        return;
      }
    }

    final token = await FirebaseMessaging.instance.getToken();
    if (token == null) {
      return;
    }

    await client.request(
      GPushNotification_RegisterPushNotificationToken_MutationReq((b) {
        b.vars.input.token = token;
      }),
    );
  } on Exception {
    // pass
  }
}

Future<void> _deleteToken() async {
  try {
    await FirebaseMessaging.instance.deleteToken();
  } on Exception {
    // pass
  }
}
