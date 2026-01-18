# Frontend Style Guide

This document defines the UI style for Woodeye's Svelte frontend, based on a modern SaaS dashboard aesthetic.

## Color Palette

### CSS Variables

```css
:root {
  /* Background */
  --color-bg: #f5f7fa;
  --color-bg-sidebar: #1a1f37;
  --color-bg-card: #ffffff;

  /* Text */
  --color-text: #1e293b;
  --color-text-muted: #64748b;
  --color-text-sidebar: #94a3b8;
  --color-text-sidebar-active: #ffffff;

  /* Accent */
  --color-primary: #7c5cfc;
  --color-primary-light: #ede9fe;
  --color-success: #22c55e;
  --color-warning: #f59e0b;
  --color-error: #f87171;
  --color-info: #14b8a6;

  /* Borders & Shadows */
  --color-border: #e5e7eb;
  --shadow-sm: 0 1px 2px rgba(0,0,0,0.05);
  --shadow-md: 0 4px 6px -1px rgba(0,0,0,0.1);
  --radius-sm: 8px;
  --radius-md: 12px;
  --radius-lg: 16px;
}
```

## Design Principles

### Layout
- **Card-based design** - Content grouped in white cards with subtle shadows
- **Dark sidebar** - Navigation on left with dark navy background
- **Generous whitespace** - 24px+ gaps between major sections
- **Three-column potential** - Sidebar | List | Detail view

### Cards
- Background: `var(--color-bg-card)` (white)
- Border radius: `var(--radius-md)` (12px)
- Shadow: `var(--shadow-md)`
- Padding: 16-24px
- No visible borders, shadows define edges

### Typography
- Font family: System fonts (-apple-system, BlinkMacSystemFont, "Segoe UI", etc.)
- Headings: Semi-bold (600), dark text
- Body: Regular (400), `var(--color-text)`
- Muted text: `var(--color-text-muted)`
- Large numbers/stats: Bold (700), 24-32px

### Sidebar Navigation
- Background: `var(--color-bg-sidebar)` (#1a1f37)
- Text: `var(--color-text-sidebar)` (muted gray)
- Active item: `var(--color-text-sidebar-active)` (white) with purple accent
- Icons paired with labels
- Hover: Subtle background highlight

### Status Indicators
- Success/Added: `var(--color-success)` - green
- Warning/Modified: `var(--color-warning)` - amber
- Error/Deleted: `var(--color-error)` - coral red
- Info: `var(--color-info)` - teal

### Interactive Elements
- Buttons: Rounded (8px), primary uses `var(--color-primary)`
- Hover states: Subtle background color shift
- Active/selected: Purple accent or background highlight
- Focus: Visible outline for accessibility

## Component Patterns

### Stat Cards
```
┌──────────────────────┐
│ Label (muted)        │
│ Large Number (bold)  │
│ [Mini chart/trend]   │
└──────────────────────┘
```

### List Items
```
┌──────────────────────────────────────┐
│ [Icon] Title                 [Badge] │
│        Subtitle (muted)              │
└──────────────────────────────────────┘
```
- Selected state: Light purple background or left border accent
- Hover: Subtle gray background

### Diff Hunks
- File headers: Collapsible, bold filename
- Added lines: Light green background
- Removed lines: Light red background
- Context: No background
- Line numbers: Muted, monospace

## Spacing Scale
- 4px - Tight (inline elements)
- 8px - Compact (related items)
- 12px - Default (list items)
- 16px - Comfortable (card padding)
- 24px - Spacious (section gaps)
- 32px - Large (major sections)

## Responsive Behavior
- Sidebar: Collapsible to icons only on narrow screens
- Cards: Stack vertically on mobile
- Diff view: Full width, horizontal scroll for long lines
