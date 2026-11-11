import 'package:flutter/material.dart';

import '../core/core_api.dart';
import '../state/app_model.dart';
import 'setup_screen.dart';

/// Which clubs are in the bag, and a loft on each if known (D7). [M38 P19]
///
/// First-run setup's second step, with [onDone], and P27's bag row without it. Every club a bag can
/// hold is listed in bag order, and ticking one is what puts it in the club picker. **A loft is shown
/// as Rust spelled it** (`loft_display`, rule 8) and typed in a dialog, so this screen never writes a
/// number out itself, and it asks a loft for no range: `set_bag_slot` asks only that it is finite, by
/// the user's call on P5, and its refusal is shown under the field.
class BagScreen extends StatelessWidget {
  const BagScreen({super.key, required this.model, this.onDone});

  final AppModel model;

  /// Setup's last step. Offered once the bag holds a club, since an empty bag is what opens setup on
  /// this screen.
  final VoidCallback? onDone;

  @override
  Widget build(BuildContext context) {
    return ListenableBuilder(
      listenable: model,
      builder: (context, _) {
        final golfer = model.golfer!;
        final slots = {for (final slot in model.bag) slot.club: slot};
        return Scaffold(
          appBar: AppBar(title: const Text('Your bag')),
          body: SafeArea(
            child: ListView(
              children: [
                ListTile(
                  title: Text(golfer.displayName),
                  subtitle: Text(
                    model.keptHandedness
                        ? 'Already known as ${handednessWord(golfer.handedness)}, and that stands.'
                        : handednessWord(golfer.handedness),
                  ),
                ),
                const Padding(
                  padding: EdgeInsets.fromLTRB(16, 8, 16, 8),
                  child: Text('Tick the clubs you carry. A loft is optional.'),
                ),
                for (final club in model.clubs)
                  _ClubRow(model: model, club: club, slot: slots[club]),
              ],
            ),
          ),
          bottomNavigationBar: onDone == null
              ? null
              : SafeArea(
                  child: Padding(
                    padding: const EdgeInsets.all(16),
                    child: FilledButton(
                      onPressed: model.bag.isEmpty ? null : onDone,
                      child: const Text('Done'),
                    ),
                  ),
                ),
        );
      },
    );
  }
}

/// One club: ticked if in the bag, and its loft beside it if it is.
class _ClubRow extends StatelessWidget {
  const _ClubRow({required this.model, required this.club, required this.slot});

  final AppModel model;
  final String club;

  /// None when the club is not in the bag.
  final BagSlot? slot;

  Future<void> _toggle(BuildContext context) async {
    try {
      if (slot == null) {
        await model.setSlot(club, null);
      } else {
        await model.removeSlot(club);
      }
    } on Refusal catch (refusal) {
      model.logProblem('bag', refusal.message);
      if (context.mounted) {
        ScaffoldMessenger.of(context)
            .showSnackBar(SnackBar(content: Text(refusal.message)));
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final slot = this.slot;
    return ListTile(
      key: ValueKey('club-$club'),
      leading: Checkbox(
        value: slot != null,
        onChanged: (_) => _toggle(context),
      ),
      title: Text(club),
      onTap: () => _toggle(context),
      trailing: slot == null
          ? null
          : TextButton(
              onPressed: () => showDialog<void>(
                context: context,
                builder: (_) => LoftDialog(model: model, slot: slot),
              ),
              child: Text(slot.loftDisplay ?? 'Add loft'),
            ),
    );
  }
}

/// A club's loft, typed. Empty is "no loft", which is what the store holds for a loft nobody has said.
class LoftDialog extends StatefulWidget {
  const LoftDialog({super.key, required this.model, required this.slot});

  final AppModel model;
  final BagSlot slot;

  @override
  State<LoftDialog> createState() => _LoftDialogState();
}

class _LoftDialogState extends State<LoftDialog> {
  final _loft = TextEditingController();
  String? _problem;
  bool _busy = false;

  @override
  void dispose() {
    _loft.dispose();
    super.dispose();
  }

  Future<void> _save() async {
    final text = _loft.text.trim();
    final double? loft;
    if (text.isEmpty) {
      loft = null;
    } else {
      loft = double.tryParse(text);
      if (loft == null) {
        // Not a number at all, so there is nothing to pass. Whether a number is a loft is Rust's
        // question, and `NaN` or `Infinity`, which do parse, go to it and are refused there.
        setState(() => _problem = 'Type the loft in degrees, as a number.');
        return;
      }
    }
    setState(() {
      _busy = true;
      _problem = null;
    });
    try {
      await widget.model.setSlot(widget.slot.club, loft);
      if (mounted) {
        Navigator.of(context).pop();
      }
    } on Refusal catch (refusal) {
      widget.model.logProblem('bag', refusal.message);
      if (mounted) {
        setState(() => _problem = refusal.message);
      }
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final current = widget.slot.loftDisplay;
    return AlertDialog(
      title: Text('${widget.slot.club} loft'),
      content: TextField(
        controller: _loft,
        autofocus: true,
        keyboardType: const TextInputType.numberWithOptions(decimal: true),
        textInputAction: TextInputAction.done,
        onSubmitted: _busy ? null : (_) => _save(),
        decoration: InputDecoration(
          labelText: 'Degrees',
          helperText: current == null
              ? 'Leave empty if not known.'
              : 'Now $current. Empty clears it.',
          errorText: _problem,
          errorMaxLines: 3,
          border: const OutlineInputBorder(),
        ),
      ),
      actions: [
        TextButton(
          onPressed: _busy ? null : () => Navigator.of(context).pop(),
          child: const Text('Cancel'),
        ),
        FilledButton(
          onPressed: _busy ? null : _save,
          child: const Text('Save'),
        ),
      ],
    );
  }
}
