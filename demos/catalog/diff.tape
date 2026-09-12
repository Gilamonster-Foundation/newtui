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
