import 'package:material_ui/material_ui.dart';

import 'src/native.dart';
import 'src/start_screen.dart';
import 'src/state.dart';
import 'src/theme.dart';
import 'src/workspace.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await Core.init();
  final model = AppModel();
  await model.init();
  runApp(KernTeX(model));
}

class KernTeX extends StatelessWidget {
  const KernTeX(this.m, {super.key});
  final AppModel m;

  @override
  Widget build(BuildContext context) {
    return ListenableBuilder(
      listenable: m,
      builder: (context, _) => MaterialApp(
        title: m.name == null ? 'KernTeX' : '${m.name} — KernTeX',
        debugShowCheckedModeBanner: false,
        themeMode: m.themeMode,
        theme: buildTheme(Brightness.light),
        darkTheme: buildTheme(Brightness.dark),
        home: Builder(
          builder: (context) => CallbackShortcuts(
            bindings: shortcuts(context, m),
            child: Focus(autofocus: true, child: Scaffold(body: m.root == null ? StartScreen(m) : Workspace(m))),
          ),
        ),
      ),
    );
  }
}
