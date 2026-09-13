Output demos/catalog/linked_panes.gif

Set Shell "bash"
Set Width 1500
Set Height 940
Set FontSize 18
Set FontFamily "Menlo"
Set Theme "Catppuccin Mocha"
Set CursorBlink false
Set Padding 16

Hide
Type `"$NEWTUI_CATALOG_BIN" --item linked_panes --width 88 --theme dark`
Enter
Wait+Screen /LIVE CATALOG/
Sleep 1s
Show
Enter
Down 2
Sleep 1s
Screenshot docs/widgets/generated/linked-panes.png
Sleep 1s
Tab
Sleep 800ms
Tab
Down 4
Sleep 1s
Screenshot docs/widgets/generated/linked-panes-anchor.png
Sleep 1s
Down 2
Sleep 1s
Type "l"
Sleep 1s
Type "l"
Up 2
Sleep 1s
Escape
Type "rf"
Enter
Left 2
Sleep 1s
Screenshot docs/widgets/generated/linked-panes-narrow.png
Sleep 1s
Escape
Type "f"
Enter
Sleep 1s
Escape
Type "f"
Enter
Sleep 1s
Escape
Type "f"
Enter
PageDown 2
Sleep 1s
Escape
Type "q"
Sleep 200ms
