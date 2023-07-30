import 'dart:async';
import 'dart:ffi';
import 'dart:io';

import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import 'package:ffi/ffi.dart';
import 'warp2_generated.dart';

final warp2_lib = init();

NativeLibrary init() {
  var lib = NativeLibrary(WarpFFI.open());
  lib.dart_post_cobject(NativeApi.postCObject.cast());
  return lib;
}

Pointer<Int8> toNative(String s) {
  return s.toNativeUtf8().cast<Int8>();
}

class WarpFFI {
  static const MethodChannel _channel = const MethodChannel('warp_ffi');

  static Future<String?> get platformVersion async {
    final String? version = await _channel.invokeMethod('getPlatformVersion');
    return version;
  }

  static DynamicLibrary open() {
    if (Platform.isAndroid) return DynamicLibrary.open('libwarp2.so');
    if (Platform.isMacOS) return DynamicLibrary.open('libwarp2.dylib');
    throw UnsupportedError('This platform is not supported.');
  }

  static Future<int> warp2Scan(String url, String fvk, int port) async {
    return await compute((_) {
      return warp2_lib.full_scan(toNative(url), toNative(fvk), port);
    }, null);
  }
}
