# Capture reproduction inputs

Generated from the real terminal host using the checked-in tapes below.

Source revision: 518184f0e9698a2990b67abc07bfd59cb9e7c7f1
Recorder: vhs version v0.11.0 (c6af91a)
Working tree: includes the reviewed changes accompanying these captures.

## demos/catalog/catalog.tape

```text
Output demos/catalog/catalog.gif

Set Shell "bash"
Set Width 1200
Set Height 760
Set FontSize 18
Set FontFamily "Menlo"
Set Theme "Catppuccin Mocha"
Set CursorBlink false
Set Padding 16

Hide
Type `"$NEWTUI_CATALOG_BIN" --item sparkline --scenario long --theme dark`
Enter
Wait+Screen /LIVE CATALOG/
Sleep 1s
Show
Sleep 1s
Screenshot docs/widgets/generated/catalog.png
Sleep 1s
Type "f"
Down
Sleep 800ms
Screenshot docs/widgets/generated/butterfly.png
Sleep 1s
Down
Sleep 800ms
Screenshot docs/widgets/generated/heat-meter.png
Sleep 1s
Down
Sleep 800ms
Screenshot docs/widgets/generated/gauge.png
Sleep 1s
Type "q"
Sleep 200ms
```

## demos/catalog/diff.tape

```text
Output demos/catalog/diff.gif

Set Shell "bash"
Set Width 1500
Set Height 940
Set FontSize 18
Set FontFamily "Menlo"
Set Theme "Catppuccin Mocha"
Set CursorBlink false
Set Padding 16

Hide
Type `"$NEWTUI_CATALOG_BIN" --item diff --width 88 --theme dark`
Enter
Wait+Screen /LIVE CATALOG/
Sleep 1s
Show
Enter
Sleep 1s
Screenshot docs/widgets/generated/diff-unified.png
Sleep 1s
Type "g"
Sleep 1s
Screenshot docs/widgets/generated/diff-split.png
Sleep 1s
Type "g"
Sleep 1s
Screenshot docs/widgets/generated/diff-stat.png
Sleep 1s
Type "g"
Escape
Type "ffff"
Enter
PageDown
Sleep 1s
Screenshot docs/widgets/generated/diff-unicode.png
Sleep 1s
Left 12
Sleep 1s
Screenshot docs/widgets/generated/diff-tiny.png
Sleep 1s
Escape
Type "q"
Sleep 200ms
```

## demos/catalog/error.tape

```text
Output demos/catalog/error.gif

Set Shell "bash"
Set Width 1200
Set Height 760
Set FontSize 18
Set FontFamily "Menlo"
Set Theme "Catppuccin Mocha"
Set CursorBlink false
Set Padding 16

Hide
Type `"$NEWTUI_CATALOG_BIN" --item settings_panel --scenario error --theme dark`
Enter
Wait+Screen /LIVE CATALOG/
Sleep 1s
Show
Sleep 1s
Screenshot docs/widgets/generated/catalog-error.png
Sleep 1s
Type "q"
Sleep 200ms
```

## demos/catalog/light.tape

```text
Output demos/catalog/light.gif

Set Shell "bash"
Set Width 1200
Set Height 760
Set FontSize 18
Set FontFamily "Menlo"
Set Theme "Catppuccin Latte"
Set CursorBlink false
Set Padding 16

Hide
Type `"$NEWTUI_CATALOG_BIN" --item settings_panel --scenario normal --theme light`
Enter
Wait+Screen /LIVE CATALOG/
Sleep 1s
Show
Sleep 1s
Screenshot docs/widgets/generated/catalog-light.png
Sleep 1s
Type "q"
Sleep 200ms
```

## demos/catalog/narrow.tape

```text
Output demos/catalog/narrow.gif

Set Shell "bash"
Set Width 760
Set Height 720
Set FontSize 18
Set FontFamily "Menlo"
Set Theme "Catppuccin Mocha"
Set CursorBlink false
Set Padding 16

Hide
Type `"$NEWTUI_CATALOG_BIN" --item butterfly --scenario narrow --theme dark`
Enter
Wait+Screen /LIVE CATALOG/
Sleep 1s
Show
Sleep 1s
Screenshot docs/widgets/generated/catalog-narrow.png
Sleep 1s
Type "q"
Sleep 200ms
```
