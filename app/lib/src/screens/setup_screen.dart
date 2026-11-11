import 'package:flutter/material.dart';

import '../core/core_api.dart';
import '../state/app_model.dart';

/// First-run setup's first step: who is swinging. [M38 P19]
///
/// **The name is not checked here.** `create_golfer` is what decides whether a name can key a golfer,
/// and its refusal is shown under the field as Rust wrote it, so this screen holds no copy of the slug
/// rule to drift from (rule 8). A name that folds to a known golfer is that golfer, and the bag screen
/// says so if the stored handedness is not the one ticked here.
///
/// **Settings' Add a golfer is this screen too** (P27), pushed over the tabs with its own [create], so
/// the name and the hand are asked one way wherever a golfer is made.
class SetupScreen extends StatefulWidget {
  const SetupScreen({super.key, required this.model, this.create});

  final AppModel model;

  /// What Continue does with the name and the hand: setup's own step, [AppModel.createGolfer], when
  /// none is given. It throws [Refusal] to keep the screen up with Rust's sentence under the field.
  final Future<void> Function(String name, Handedness handedness)? create;

  @override
  State<SetupScreen> createState() => _SetupScreenState();
}

class _SetupScreenState extends State<SetupScreen> {
  final _name = TextEditingController();
  Handedness _handedness = Handedness.right;
  String? _refusal;
  bool _busy = false;

  @override
  void dispose() {
    _name.dispose();
    super.dispose();
  }

  Future<void> _continue() async {
    setState(() {
      _busy = true;
      _refusal = null;
    });
    try {
      final create = widget.create ?? widget.model.createGolfer;
      await create(_name.text, _handedness);
    } on Refusal catch (refusal) {
      widget.model.logProblem('setup', refusal.message);
      if (mounted) {
        setState(() => _refusal = refusal.message);
      }
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final text = Theme.of(context).textTheme;
    return Scaffold(
      appBar: AppBar(title: const Text('Golf Coach')),
      body: SafeArea(
        child: ListView(
          padding: const EdgeInsets.all(24),
          children: [
            Text('Who is swinging?', style: text.headlineSmall),
            const SizedBox(height: 24),
            TextField(
              controller: _name,
              autofocus: true,
              textCapitalization: TextCapitalization.words,
              textInputAction: TextInputAction.done,
              onSubmitted: _busy ? null : (_) => _continue(),
              decoration: InputDecoration(
                labelText: 'Name',
                border: const OutlineInputBorder(),
                errorText: _refusal,
                errorMaxLines: 3,
              ),
            ),
            const SizedBox(height: 24),
            Text('Swings', style: text.titleMedium),
            const SizedBox(height: 8),
            SegmentedButton<Handedness>(
              segments: const [
                ButtonSegment(
                  value: Handedness.right,
                  label: Text('Right-handed'),
                ),
                ButtonSegment(
                  value: Handedness.left,
                  label: Text('Left-handed'),
                ),
              ],
              selected: {_handedness},
              onSelectionChanged: (picked) =>
                  setState(() => _handedness = picked.single),
            ),
            const SizedBox(height: 32),
            FilledButton(
              onPressed: _busy ? null : _continue,
              child: const Text('Continue'),
            ),
          ],
        ),
      ),
    );
  }
}

/// How a handedness reads in a sentence.
String handednessWord(Handedness handedness) => switch (handedness) {
  Handedness.right => 'right-handed',
  Handedness.left => 'left-handed',
};
