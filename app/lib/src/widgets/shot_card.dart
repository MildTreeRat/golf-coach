import 'package:flutter/material.dart';

import '../core/core_api.dart';

/// One stored shot as the card every read shows (D9), and the History tab's after it. [M38 P21]
///
/// **Every word and number on it is Rust's** (rule 8): each row's label and display, the warnings, and
/// whether it is flagged. What this adds is layout and three plain labels. A row Rust could not read
/// has no display, and it shows as not read on a highlighted row and never as 0 (ADR-010 §2), so a
/// golfer looking from the photo to the card sees at once which tile to check. A tile the device does
/// not print has no row at all, since `card_of` builds rows only for what the screen declares.
///
/// The card is keyed `card-<session>-<swing>` and each row `row-<field>`, so a test can find the shot
/// a card shows and a row by its `ShotData` key.
class ShotCardView extends StatelessWidget {
  ShotCardView({required this.card, this.message, this.onTap})
    : super(key: ValueKey('card-${card.sessionId}-${card.swingId}'));

  final ShotCard card;

  /// Rust's sentence above the card, as a duplicate's answer carries it.
  final String? message;

  /// What a tap on the card does: on the Record tab, open its review (P22). None is a card that does
  /// nothing when tapped.
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colors = theme.colorScheme;
    final message = this.message;
    return Card(
      margin: EdgeInsets.zero,
      clipBehavior: Clip.antiAlias,
      child: InkWell(
        onTap: onTap,
        child: Padding(
          padding: const EdgeInsets.symmetric(vertical: 12),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              if (message != null)
                Padding(
                  padding: const EdgeInsets.fromLTRB(16, 0, 16, 8),
                  child: Text(message, style: theme.textTheme.bodyMedium),
                ),
              Padding(
                padding: const EdgeInsets.symmetric(horizontal: 16),
                child: Row(
                  children: [
                    Expanded(
                      child: Text(
                        'Shot ${card.swingId}',
                        style: theme.textTheme.titleMedium,
                      ),
                    ),
                    if (card.flagged)
                      Chip(
                        key: const ValueKey('flagged'),
                        avatar: Icon(
                          Icons.flag,
                          color: colors.onErrorContainer,
                        ),
                        label: const Text('Needs review'),
                        backgroundColor: colors.errorContainer,
                        labelStyle: TextStyle(color: colors.onErrorContainer),
                      ),
                  ],
                ),
              ),
              for (final warning in card.warnings)
                Padding(
                  padding: const EdgeInsets.fromLTRB(16, 8, 16, 0),
                  child: Text(
                    warning,
                    style: theme.textTheme.bodySmall?.copyWith(
                      color: colors.error,
                    ),
                  ),
                ),
              const SizedBox(height: 8),
              for (final row in card.rows) CardRowView(row: row),
            ],
          ),
        ),
      ),
    );
  }
}

/// One tile: its label, and what was read or that nothing was.
class CardRowView extends StatelessWidget {
  CardRowView({required this.row}) : super(key: ValueKey('row-${row.field}'));

  final CardRow row;

  /// What a row nothing was read for says in place of a number.
  static const notRead = 'not read';

  /// What a row says whose value a person cleared, since they have looked and it is not the photo's
  /// doing (`card_of` calls that `Corrected` and not `NotRead`).
  static const cleared = 'cleared';

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colors = theme.colorScheme;
    final missing = row.state == RowState.notRead;
    final foreground = missing ? colors.onErrorContainer : null;
    return Container(
      color: missing ? colors.errorContainer : null,
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 6),
      child: Row(
        children: [
          Expanded(
            child: Text(
              row.label,
              style: theme.textTheme.bodyLarge?.copyWith(color: foreground),
            ),
          ),
          if (row.state == RowState.corrected)
            Padding(
              padding: const EdgeInsets.only(right: 8),
              child: Text(
                'corrected',
                style: theme.textTheme.labelSmall?.copyWith(
                  color: colors.tertiary,
                ),
              ),
            ),
          Text(
            row.display ??
                (row.state == RowState.corrected ? cleared : notRead),
            style: theme.textTheme.titleMedium?.copyWith(
              color: foreground,
              fontStyle: row.display == null ? FontStyle.italic : null,
            ),
          ),
        ],
      ),
    );
  }
}
