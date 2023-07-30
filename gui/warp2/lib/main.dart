import 'dart:ffi';
import 'dart:isolate';

import 'package:flutter/material.dart';
import 'package:flutter_form_builder/flutter_form_builder.dart';
import 'package:warp2/warp2_ffi.dart';

var syncPort = ReceivePort();
var syncStream = syncPort.asBroadcastStream();

void main() {
  runApp(const MyApp());
}

class MyApp extends StatelessWidget {
  const MyApp({super.key});

  // This widget is the root of your application.
  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Warp2 Demo',
      theme: ThemeData(
        colorScheme: ColorScheme.fromSeed(seedColor: Colors.deepPurple),
        useMaterial3: true,
      ),
      home: const MyHomePage(),
    );
  }
}

class MyHomePage extends StatefulWidget {
  const MyHomePage({super.key});

  @override
  State<MyHomePage> createState() => _MyHomePageState();
}

class _MyHomePageState extends State<MyHomePage> {
  int balance = 0;
  int? height;
  int? elapsed;
  TextEditingController url = TextEditingController(text: 'http://192.168.0.158:8080/compact.dat');
  TextEditingController fvk = TextEditingController(text: 'zxviews1q0duytgcqqqqpqre26wkl45gvwwwd706xw608hucmvfalr759ejwf7qshjf5r9aa7323zulvz6plhttp5mltqcgs9t039cx2d09mgq05ts63n8u35hyv6h9nc9ctqqtue2u7cer2mqegunuulq2luhq3ywjcz35yyljewa4mgkgjzyfwh6fr6jd0dzd44ghk0nxdv2hnv4j5nxfwv24rwdmgllhe0p8568sgqt9ckt02v2kxf5ahtql6s0ltjpkckw8gtymxtxuu9gcr0swvz');

  @override
  void initState() {
    super.initState();
    syncStream.forEach((height) {
      this.height = height;
      setState(() {});
    });
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        backgroundColor: Theme.of(context).colorScheme.inversePrimary,
        title: const Text('Warp 2 Demo'),
      ),
      body: Padding(padding: EdgeInsets.all(32), child: Center(
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: <Widget>[
            FormBuilderTextField(
                decoration:
                const InputDecoration(labelText: 'data file url'),
                name: 'url',
                controller: url),
            FormBuilderTextField(
                decoration:
                const InputDecoration(labelText: 'full viewing key'),
                name: 'fvk',
                minLines: 8,
                maxLines: 8,
                controller: fvk),
            Text(
              'Your balance is: $balance',
            ),
            if (height != null) Text(
              'Current height: $height',
            ),
            if (elapsed != null) Text(
              'Scan duration: $elapsed secs',
            ),
          ],
        ),
      ),
      ),
      floatingActionButton: FloatingActionButton(
        onPressed: _warp,
        child: const Icon(Icons.play_arrow),
      ), // This trailing comma makes auto-formatting nicer for build methods.
    );
  }

  _warp() async {
    elapsed = null;
    height = null;
    final stopwatch = Stopwatch()..start();
    balance = await WarpFFI.warp2Scan(url.text, fvk.text, syncPort.sendPort.nativePort);
    elapsed = stopwatch.elapsed.inSeconds;
    setState(() {});
  }
}
