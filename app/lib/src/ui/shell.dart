import 'package:flutter/material.dart';

import '../screens/history_screen.dart';
import '../screens/profile_screen.dart';
import '../screens/record_screen.dart';
import '../screens/settings_screen.dart';
import '../screens/shot_detail_screen.dart';
import '../state/app_model.dart';

/// The four tabs (D13), Record first. [M38 P19]
///
/// Each tab is kept alive in an [IndexedStack], so a half-typed field or a scrolled list survives a
/// look at another tab, and the Record tab's last card is still there after one. Each began as a
/// placeholder, and P21, P24, P26 and P27 replaced Record's, History's, Profile's and Settings' with
/// their screens. A History row opens P25's detail screen over the tabs.
class Shell extends StatefulWidget {
  const Shell({super.key, required this.model});

  final AppModel model;

  @override
  State<Shell> createState() => _ShellState();
}

/// The tabs in the order the bar shows them.
enum ShellTab {
  record('Record', Icons.photo_camera_outlined, Icons.photo_camera),
  history('History', Icons.history_outlined, Icons.history),
  profile('Profile', Icons.insights_outlined, Icons.insights),
  settings('Settings', Icons.settings_outlined, Icons.settings);

  const ShellTab(this.label, this.icon, this.selectedIcon);

  final String label;
  final IconData icon;
  final IconData selectedIcon;
}

class _ShellState extends State<Shell> {
  ShellTab _tab = ShellTab.record;

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      body: SafeArea(
        child: IndexedStack(
          index: _tab.index,
          children: [
            for (final tab in ShellTab.values)
              switch (tab) {
                ShellTab.record => RecordScreen(model: widget.model),
                // Told whether it is the tab shown, since entering it is when it reads the stores
                // again, and the bar's tap below is the only place a tab changes (P19 finding 10).
                ShellTab.history => HistoryScreen(
                  model: widget.model,
                  active: _tab == ShellTab.history,
                  openShot: (context, row) =>
                      showShotDetail(context, widget.model, row),
                ),
                // Told the same, for the same reason: a grade moves with every shot written.
                ShellTab.profile => ProfileScreen(
                  model: widget.model,
                  active: _tab == ShellTab.profile,
                ),
                // Told the same, since the shots since the last export move with every shot written.
                ShellTab.settings => SettingsScreen(
                  model: widget.model,
                  active: _tab == ShellTab.settings,
                ),
              },
          ],
        ),
      ),
      bottomNavigationBar: NavigationBar(
        selectedIndex: _tab.index,
        onDestinationSelected: (index) =>
            setState(() => _tab = ShellTab.values[index]),
        destinations: [
          for (final tab in ShellTab.values)
            NavigationDestination(
              icon: Icon(tab.icon),
              selectedIcon: Icon(tab.selectedIcon),
              label: tab.label,
            ),
        ],
      ),
    );
  }
}
