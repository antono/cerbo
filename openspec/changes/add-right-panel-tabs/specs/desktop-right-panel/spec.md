# Spec Delta

## Purpose

The right panel holds several sub-panels and shows one at a time, chosen from a strip of small icons along its top edge.

## ADDED Requirements

### Requirement: Icon tab strip

The right panel SHALL show a strip of icon buttons along its top edge, one per sub-panel, in this order: Attachments, Links, Outline, Agent. Exactly one tab SHALL be selected, and only its sub-panel SHALL be shown. Each icon SHALL have an accessible name and a tooltip with the tab name and its hotkey.

#### Scenario: Switch tab
- **WHEN** the user clicks the Links icon
- **THEN** the Links sub-panel SHALL replace the one shown
- **THEN** the Links icon SHALL be marked selected and the others not

#### Scenario: Tooltip
- **WHEN** the user hovers the Outline icon
- **THEN** a tooltip SHALL show "Outline" and its hotkey

### Requirement: Tab badges

The Attachments icon SHALL show the number of attachments of the open page when it is above zero. The Agent icon SHALL show a status mark: connected, working, waiting for approval, or not connected. A badge SHALL show on a tab that is not selected too.

#### Scenario: Attachment count
- **WHEN** the open page has 3 attachments and Links is selected
- **THEN** the Attachments icon SHALL show 3

### Requirement: Selected tab is remembered

The selected tab SHALL stay the same when the open page changes. It SHALL be saved with the UI settings and restored when the app starts. An unknown saved value SHALL fall back to Links.

#### Scenario: Navigate keeps the tab
- **WHEN** Outline is selected and the user opens another page
- **THEN** Outline SHALL stay selected and show the new page's headings

#### Scenario: Restart
- **WHEN** the user quits with Attachments selected and starts the app again
- **THEN** Attachments SHALL be selected

### Requirement: Tab hotkeys and commands

Each tab SHALL have a hotkey and a command-palette action named "Show <tab>". Using it SHALL show the right panel if hidden, then select the tab. Using it for the selected tab while the panel is shown SHALL hide the panel.

#### Scenario: Hotkey opens hidden panel
- **WHEN** the right panel is hidden and the user presses the Outline hotkey
- **THEN** the panel SHALL show with Outline selected

#### Scenario: Hotkey toggles
- **WHEN** the panel is shown with Outline selected and the user presses the Outline hotkey
- **THEN** the panel SHALL hide

### Requirement: Same tabs in every layout

The docked panel (wide), the floating panel (medium) and the bottom sheet (narrow) SHALL all use the same tab strip and the same selected tab. The sheet SHALL NOT have its own tab bar.

#### Scenario: Narrow layout
- **WHEN** the window is narrow and the user opens the right panel
- **THEN** the sheet SHALL show the icon strip with the selected tab's sub-panel

### Requirement: Sub-panels scroll on their own

Each sub-panel SHALL scroll within itself. The tab strip SHALL stay visible while the sub-panel scrolls. Switching tabs and back SHALL keep each sub-panel's scroll position while the same page stays open.

#### Scenario: Scroll kept
- **WHEN** the user scrolls the Links list, switches to Outline and back
- **THEN** the Links list SHALL be at the same scroll position
