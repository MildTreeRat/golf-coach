import 'dart:async';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../core/core_api.dart';
import '../state/app_model.dart';
import 'bag_screen.dart';
import 'setup_screen.dart';

/// The Settings tab: who is swinging, their bag, the app's look, how the shots leave the phone, and
/// which build this is (D13). [M38 P27]
///
/// **Export is three calls in a fixed order** (P13 finding 13): `export_sessions` cuts the zip, the
/// share sheet sends it, and only then does `set_state` record the summary's own `exported_at` and
/// `shots` as the last export. So the shots since are counted from when the zip was cut and not from
/// when the sheet closed, and **a sheet closed with nothing sent records nothing**: no file left the
/// phone, so the shots it held are still unsent, which is what the count here is for. The zip was cut
/// all the same, and the next export cuts it again, since each one holds every shot (C16).
///
/// **Every count and time on it is Rust's** (rule 8): `export_status` answers the last export as
/// `2026-11-05 11:00`, its shots, the shots on the phone and the shots since, and this subtracts
/// nothing. `about` answers the build.
///
/// It reads `export_status` on entering the tab, after an export, and while it is the tab shown after
/// every shot written ([AppModel.shotsChanged]), since a library batch runs on while the golfer looks
/// here. `about` is read once, on the first entry, since a build does not change under a running app.
class SettingsScreen extends StatefulWidget {
  const SettingsScreen({super.key, required this.model, required this.active});

  final AppModel model;

  /// Whether this is the tab shown. Becoming it is what reads the export's status again.
  final bool active;

  /// Dart's own sentences and labels, under rule 8: none is about a value.
  static const title = 'Settings';

  static const addGolfer = 'Add a golfer';

  static const emptyBag = 'No club in the bag yet.';

  static const bagHint =
      'Which clubs you carry, and a loft on each if you know it.';

  static const noGolfer = 'No golfer is chosen.';

  static const neverExported = 'Nothing has been exported from this phone yet.';

  static const cumulative =
      'Each export holds every shot on this phone, so the newest one is all the lab needs.';

  static const exportLabel = 'Export and share';

  static const notShared =
      'The share sheet closed with nothing sent, so this does not count as the last export.';

  static const deleteWarning =
      'Deleting Golf Coach from this phone deletes every shot it holds, and nothing brings them '
      'back. Export first. Installing a new build over the app keeps them.';

  static String shotCount(int count) => count == 1 ? '1 shot' : '$count shots';

  /// The last export, as `export_status` spells its time and counts its shots.
  static String lastExport(String display, int? shots) => shots == null
      ? 'Last export $display.'
      : 'Last export $display, holding ${shotCount(shots)}.';

  /// The shots on the phone, and, once there has been an export, those opened since it.
  static String onPhone(ExportStatus status) => status.lastExportDisplay == null
      ? '${shotCount(status.shots)} on this phone.'
      : '${shotCount(status.shots)} on this phone, ${status.shotsSince} new since then.';

  @override
  State<SettingsScreen> createState() => _SettingsScreenState();
}

/// Where an export's zip goes: `tmp/export/` beside `Documents/`, named for the install and the time
/// it was cut. [M38 P27]
///
/// **Taken from the data root, and so from the one `documentsDir` answer** it was built from (P13
/// finding 13): `export_sessions` refuses a path inside the root by comparing the two as written, and
/// two answers could spell one directory two ways. On iOS the directory beside `Documents/` named `tmp`
/// is the app's own temporary directory, the one `PhotoSource` copies photos into, and iOS purges it.
/// It is not under the root, so the next export cannot carry this one.
///
/// The name is the install id, which no other phone shares (C8), and [now]'s wall clock with its
/// colons taken out, which macOS shows as slashes. Nothing at the lab reads the name, and P31's run
/// sheet saves the zip under the session's date.
String exportPath(String root, String installId, String now) {
  final container = Directory(root).parent.parent.path;
  final stamp = now.substring(0, 19).replaceAll(':', '').replaceAll('T', '-');
  return '$container/tmp/export/golf-coach-$installId-$stamp.zip';
}

class _SettingsScreenState extends State<SettingsScreen> {
  ExportStatus? _status;
  AboutView? _about;

  /// Each status read's number. Only the latest's answer is shown, since two can be out at once.
  int _ticket = 0;

  /// The shots version the last status read asked at, so a model's change starts a read only when a
  /// shot was written, and not on every change of a setting.
  int? _askedVersion;

  bool _exporting = false;

  /// What the last export came to when it did not end in a recorded export: Rust's refusal, the share
  /// sheet's error, or [SettingsScreen.notShared]. None after one that did.
  String? _outcome;

  /// A golfer switch or an appearance change is out, so the rows that start one are held.
  bool _saving = false;

  @override
  void initState() {
    super.initState();
    widget.model.addListener(_follow);
    if (widget.active) {
      _enter();
    }
  }

  @override
  void didUpdateWidget(SettingsScreen old) {
    super.didUpdateWidget(old);
    if (old.model != widget.model) {
      old.model.removeListener(_follow);
      widget.model.addListener(_follow);
    }
    if (widget.active && !old.active) {
      _enter();
    }
  }

  @override
  void dispose() {
    widget.model.removeListener(_follow);
    super.dispose();
  }

  void _enter() {
    _readStatus();
    if (_about == null) {
      _readAbout();
    }
  }

  /// A shot written while this tab is shown. No redraw of its own: the app's root rebuilds the Shell on
  /// every change of the model, which redraws the golfer, the bag and the appearance here.
  void _follow() {
    if (widget.active && widget.model.shotsVersion != _askedVersion) {
      _readStatus();
    }
  }

  Future<void> _readStatus() async {
    final model = widget.model;
    final ticket = ++_ticket;
    _askedVersion = model.shotsVersion;
    final status = await model.core.exportStatus(root: model.root);
    if (mounted && ticket == _ticket) {
      setState(() => _status = status);
    }
  }

  Future<void> _readAbout() async {
    final about = await widget.model.core.about();
    if (mounted) {
      setState(() => _about = about);
    }
  }

  Future<void> _export() async {
    final model = widget.model;
    setState(() {
      _exporting = true;
      _outcome = null;
    });
    String? outcome;
    try {
      final now = model.now();
      final out = exportPath(model.root, model.state.installId!, now);
      // Only the latest zip is kept: one on its way out through an earlier sheet has gone by now,
      // since a sheet is up until it is done, and each zip holds every shot (C16).
      final dir = Directory(out).parent;
      if (dir.existsSync()) {
        dir.deleteSync(recursive: true);
      }
      final summary = await model.core.exportSessions(
        root: model.root,
        outPath: out,
        now: now,
      );
      final sent = await model.phone.share(summary.path);
      // After the zip was cut, so this line is in the next export and not in this one.
      model.logEvent(
        EventKind.exportShared,
        'completed $sent, ${counted(summary.shots, 'shot')}',
      );
      if (sent) {
        await model.editState(
          (state) => state.copyWith(
            lastExportAt: () => summary.exportedAt,
            lastExportShots: () => summary.shots,
          ),
        );
      } else {
        outcome = SettingsScreen.notShared;
      }
    } on Refusal catch (refusal) {
      outcome = refusal.message;
      model.logProblem('export', refusal.message);
    } on PlatformException catch (failure) {
      outcome = failure.message ?? failure.code;
      model.logProblem('share', outcome);
    } on FileSystemException catch (failure) {
      outcome = '$failure';
      model.logProblem('export', outcome);
    } finally {
      if (mounted) {
        setState(() {
          _exporting = false;
          _outcome = outcome;
        });
      }
    }
    // Read again however it ended: a refusal or a closed sheet changed nothing stored, and reading
    // costs a walk of the manifests, which is what a golfer just tapped for.
    await _readStatus();
  }

  /// Run [change], holding the rows that start one while it is out, and show a refusal in Rust's words.
  Future<void> _save(Future<void> Function() change) async {
    setState(() => _saving = true);
    try {
      await change();
    } on Refusal catch (refusal) {
      widget.model.logProblem('settings', refusal.message);
      if (mounted) {
        ScaffoldMessenger.of(context)
            .showSnackBar(SnackBar(content: Text(refusal.message)));
      }
    } finally {
      if (mounted) {
        setState(() => _saving = false);
      }
    }
  }

  Future<void> _addGolfer() async {
    final model = widget.model;
    final navigator = Navigator.of(context);
    await navigator.push(
      MaterialPageRoute<void>(
        builder: (_) => SetupScreen(
          model: model,
          create: (name, handedness) async {
            await model.addGolfer(name, handedness);
            // The bag next, as setup goes on to it, since a new golfer's club picker would list
            // nothing (D7). Not awaited: it completes when the bag screen closes.
            unawaited(
              navigator.pushReplacement(
                MaterialPageRoute<void>(
                  builder: (_) => BagScreen(model: model),
                ),
              ),
            );
          },
        ),
      ),
    );
  }

  void _openBag() {
    Navigator.of(context).push(
      MaterialPageRoute<void>(builder: (_) => BagScreen(model: widget.model)),
    );
  }

  @override
  Widget build(BuildContext context) {
    final model = widget.model;
    final theme = Theme.of(context);
    final text = theme.textTheme;
    final state = model.state;
    final golfer = model.golfer;
    return ListView(
      key: const ValueKey('settings-list'),
      padding: const EdgeInsets.symmetric(vertical: 16),
      children: [
        Padding(
          padding: const EdgeInsets.symmetric(horizontal: 16),
          child: Text(
            SettingsScreen.title,
            key: const ValueKey('settings-title'),
            style: text.headlineSmall,
          ),
        ),
        const _Heading('Golfer'),
        for (final each in model.golfers)
          ListTile(
            key: ValueKey('golfer-${each.playerId}'),
            title: Text(each.displayName),
            subtitle: Text(handednessWord(each.handedness)),
            selected: each.playerId == state.playerId,
            trailing: each.playerId == state.playerId
                ? const Icon(Icons.check)
                : null,
            onTap: _saving || each.playerId == state.playerId
                ? null
                : () => _save(() => model.chooseGolfer(each.playerId)),
          ),
        ListTile(
          key: const ValueKey('golfer-add'),
          leading: const Icon(Icons.person_add_outlined),
          title: const Text(SettingsScreen.addGolfer),
          onTap: _saving ? null : _addGolfer,
        ),
        const _Heading('Bag'),
        ListTile(
          key: const ValueKey('bag-row'),
          title: Text(
            golfer == null
                ? SettingsScreen.noGolfer
                : model.bag.isEmpty
                ? SettingsScreen.emptyBag
                : model.bag.map((slot) => slot.club).join(', '),
          ),
          subtitle: const Text(SettingsScreen.bagHint),
          trailing: const Icon(Icons.chevron_right),
          enabled: golfer != null,
          onTap: _openBag,
        ),
        const _Heading('Appearance'),
        Padding(
          padding: const EdgeInsets.symmetric(horizontal: 16),
          child: SegmentedButton<Appearance>(
            key: const ValueKey('appearance'),
            segments: const [
              ButtonSegment(value: Appearance.dark, label: Text('Dark')),
              ButtonSegment(value: Appearance.system, label: Text('System')),
              ButtonSegment(value: Appearance.light, label: Text('Light')),
            ],
            selected: {state.appearance},
            onSelectionChanged: _saving
                ? null
                : (picked) => _save(() => model.setAppearance(picked.single)),
          ),
        ),
        const _Heading('Export'),
        _Padded(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              ..._statusLines(text),
              const SizedBox(height: 8),
              Text(SettingsScreen.cumulative, style: text.bodySmall),
              const SizedBox(height: 16),
              FilledButton.icon(
                key: const ValueKey('export-button'),
                onPressed: _exporting ? null : _export,
                icon: const Icon(Icons.ios_share),
                label: const Text(SettingsScreen.exportLabel),
              ),
              if (_exporting)
                const Padding(
                  padding: EdgeInsets.only(top: 12),
                  child: LinearProgressIndicator(key: ValueKey('export-busy')),
                ),
              if (_outcome case final outcome?)
                Padding(
                  padding: const EdgeInsets.only(top: 12),
                  child: Text(
                    outcome,
                    key: const ValueKey('export-outcome'),
                    style: text.bodyMedium?.copyWith(
                      color: theme.colorScheme.error,
                    ),
                  ),
                ),
              const SizedBox(height: 16),
              Card(
                key: const ValueKey('delete-warning'),
                color: theme.colorScheme.errorContainer,
                child: Padding(
                  padding: const EdgeInsets.all(12),
                  child: Row(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Icon(
                        Icons.warning_amber_outlined,
                        color: theme.colorScheme.onErrorContainer,
                      ),
                      const SizedBox(width: 12),
                      Expanded(
                        child: Text(
                          SettingsScreen.deleteWarning,
                          style: text.bodyMedium?.copyWith(
                            color: theme.colorScheme.onErrorContainer,
                          ),
                        ),
                      ),
                    ],
                  ),
                ),
              ),
            ],
          ),
        ),
        const _Heading('About'),
        ..._aboutRows(),
      ],
    );
  }

  List<Widget> _statusLines(TextTheme text) {
    final status = _status;
    if (status == null) {
      return const [];
    }
    final display = status.lastExportDisplay;
    return [
      Text(
        display == null
            ? SettingsScreen.neverExported
            : SettingsScreen.lastExport(display, status.lastExportShots),
        key: const ValueKey('export-last'),
        style: text.bodyLarge,
      ),
      const SizedBox(height: 4),
      Text(
        SettingsScreen.onPhone(status),
        key: const ValueKey('export-shots'),
        style: text.bodyLarge,
      ),
    ];
  }

  List<Widget> _aboutRows() {
    final about = _about;
    if (about == null) {
      return const [];
    }
    return [
      for (final (key, label, value) in [
        ('about-analysis', 'Analysis version', '${about.analysisVersion}'),
        (
          'about-screen',
          'Screen parser version',
          '${about.screenParserVersion}',
        ),
        ('about-career', 'Career version', '${about.careerVersion}'),
        ('about-device', 'Launch monitor', about.device),
        ('about-build', 'Core build', about.crateVersion),
      ])
        ListTile(
          key: ValueKey(key),
          title: Text(label),
          trailing: Text(value, key: const ValueKey('value')),
        ),
    ];
  }
}

class _Heading extends StatelessWidget {
  const _Heading(this.text);

  final String text;

  @override
  Widget build(BuildContext context) => Padding(
    padding: const EdgeInsets.fromLTRB(16, 24, 16, 8),
    child: Text(text, style: Theme.of(context).textTheme.titleSmall),
  );
}

class _Padded extends StatelessWidget {
  const _Padded({required this.child});

  final Widget child;

  @override
  Widget build(BuildContext context) => Padding(
    padding: const EdgeInsets.symmetric(horizontal: 16),
    child: child,
  );
}
